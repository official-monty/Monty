mod half;
mod hash;
mod lock;
mod node;

use half::TreeHalf;
use hash::{HashEntry, HashTable};
pub use node::{Node, NodePtr};

use std::{
    mem::MaybeUninit,
    ops::Index,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    chess::{ChessState, GameState},
    mcts::{MctsParams, SearchHelpers},
    networks::PolicyNetwork,
};

#[cfg(feature = "datagen")]
use crate::chess::Move;

pub struct Tree {
    root: ChessState,
    tree: [TreeHalf; 2],
    half: AtomicBool,
    hash: HashTable,
}

impl Index<NodePtr> for Tree {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.tree[usize::from(index.half())][index]
    }
}

impl Tree {
    pub fn new_mb(mb: usize, threads: usize) -> Self {
        let bytes = mb * 1024 * 1024;

        const _: () = assert!(
            std::mem::size_of::<Node>() == 40,
            "You must reconsider this allocation!"
        );

        Self::new(bytes / 42, bytes / 42 / 16, threads)
    }

    fn new(tree_cap: usize, hash_cap: usize, threads: usize) -> Self {
        Self {
            root: ChessState::default(),
            tree: [
                TreeHalf::new(tree_cap / 2, false, threads),
                TreeHalf::new(tree_cap / 2, true, threads),
            ],
            half: AtomicBool::new(false),
            hash: HashTable::new(hash_cap / 4, threads),
        }
    }

    pub fn root_position(&self) -> &ChessState {
        &self.root
    }

    pub fn half(&self) -> usize {
        usize::from(self.half.load(Ordering::Relaxed))
    }

    pub fn is_full(&self) -> bool {
        self.tree[self.half()].is_full()
    }

    pub fn push_new_node(&self) -> Option<NodePtr> {
        self.tree[self.half()].reserve_nodes_thread(1, 0)
    }

    fn copy_node_across(&self, from: NodePtr, to: NodePtr) {
        if from == to {
            return;
        }

        let f = self[from].actions_mut();
        let t = self[to].actions_mut();

        self[to].copy_from(&self[from]);
        self[to].set_num_actions(self[from].num_actions());
        t.store(f.val());
    }

    fn copy_across(&self, from: NodePtr, num: usize, to: NodePtr) {
        for i in 0..num {
            self.copy_node_across(from + i, to + i);
        }
    }

    pub fn flip(&self, copy_across: bool, threads: usize) {
        let old_root_ptr = self.root_node();

        let old = usize::from(self.half.fetch_xor(true, Ordering::Relaxed));
        self.tree[old].clear_ptrs(threads);
        self.tree[old ^ 1].clear();

        if copy_across {
            let new_root_ptr = self.tree[self.half()].reserve_nodes_thread(1, 0).unwrap();
            self[new_root_ptr].clear();

            self.copy_node_across(old_root_ptr, new_root_ptr);
        }
    }

    #[must_use]
    pub fn fetch_children(&self, parent_ptr: NodePtr, thread_id: usize) -> Option<()> {
        let first_child_ptr = { self[parent_ptr].actions() };

        if first_child_ptr.half() != self.half.load(Ordering::Relaxed) {
            let most_recent_ptr = self[parent_ptr].actions_mut();

            if most_recent_ptr.val().half() == self.half.load(Ordering::Relaxed) {
                return Some(());
            }

            assert_eq!(first_child_ptr, most_recent_ptr.val());

            let num_children = self[parent_ptr].num_actions();
            let new_ptr = self.tree[self.half()].reserve_nodes_thread(num_children, thread_id)?;

            self.copy_across(first_child_ptr, num_children, new_ptr);

            most_recent_ptr.store(new_ptr);
        }

        Some(())
    }

    pub fn root_node(&self) -> NodePtr {
        NodePtr::new(self.half.load(Ordering::Relaxed), 0)
    }

    pub fn probe_hash(&self, hash: u64) -> Option<HashEntry> {
        self.hash.get(hash)
    }

    pub fn push_hash(&self, hash: u64, wins: f32) {
        self.hash.push(hash, wins);
    }

    fn clear_halves(&self) {
        self.tree[0].clear();
        self.tree[1].clear();
    }

    pub fn clear(&mut self, threads: usize) {
        self.root = ChessState::default();
        self.clear_halves();
        self.hash.clear(threads);
    }

    pub fn is_empty(&self) -> bool {
        self.tree[0].is_empty() && self.tree[1].is_empty()
    }

    pub fn expand_node(
        &self,
        node_ptr: NodePtr,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
        depth: usize,
        thread_id: usize,
    ) -> Option<()> {
        let node = &self[node_ptr];

        let actions_ptr = node.actions_mut();

        // when running with >1 threads, this function may
        // be called twice, and this acts as a safeguard in
        // that case
        if !node.is_not_expanded() {
            return Some(());
        }

        let mut max = f32::NEG_INFINITY;
        let mut moves = [const { MaybeUninit::uninit() }; 256];
        let mut count = 0;

        pos.map_moves_with_policies(policy, |mov, policy| {
            moves[count].write((mov, policy));
            count += 1;
            max = max.max(policy);
        });

        let new_ptr = self.tree[self.half()].reserve_nodes_thread(count, thread_id)?;

        let pst = SearchHelpers::get_pst(depth, self[node_ptr].q(), params);

        let mut total = 0.0;

        for item in moves.iter_mut().take(count) {
            let (mov, mut policy) = unsafe { item.assume_init() };
            policy = ((policy - max) / pst).exp();
            total += policy;
            item.write((mov, policy));
        }

        let mut sum_of_squares = 0.0;

        for (action, item) in moves.iter().take(count).enumerate() {
            let (mov, policy) = unsafe { item.assume_init() };
            let ptr = new_ptr + action;
            let policy = policy / total;

            self[ptr].set_new(mov, policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        node.set_gini_impurity(gini_impurity);

        actions_ptr.store(new_ptr);
        node.set_num_actions(count);

        Some(())
    }

    pub fn relabel_policy(
        &self,
        node_ptr: NodePtr,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
        depth: u8,
    ) {
        let actions = self[node_ptr].actions_mut();
        let num_actions = self[node_ptr].num_actions();
        let actions_ptr = actions.val();

        let hl = pos.get_policy_hl(policy);
        let mut max = f32::NEG_INFINITY;
        let mut policies = Vec::new();

        for action in 0..num_actions {
            let mov = self[actions_ptr + action].parent_move();
            let policy = pos.get_policy(mov, &hl, policy);

            policies.push(policy);
            max = max.max(policy);
        }

        let pst = SearchHelpers::get_pst(depth.into(), self[node_ptr].q(), params);

        let mut total = 0.0;

        for policy in &mut policies {
            *policy = ((*policy - max) / pst).exp();
            total += *policy;
        }

        let mut sum_of_squares = 0.0;

        for (action, &policy) in policies.iter().enumerate() {
            let policy = policy / total;
            self[actions_ptr + action].set_policy(policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        self[node_ptr].set_gini_impurity(gini_impurity);
    }

    pub fn propogate_proven_mates(&self, ptr: NodePtr, child_state: GameState) {
        match child_state {
            // if the child node resulted in a loss, then
            // this node has a guaranteed win
            GameState::Lost(n) => self[ptr].set_state(GameState::Won(n + 1)),
            // if the child node resulted in a win, then check if there are
            // any non-won children, and if not, guaranteed loss for this node
            GameState::Won(n) => {
                assert_ne!(self[ptr].num_actions(), 0);

                let mut proven_loss = true;
                let mut max_win_len = n;
                let first_child_ptr = self[ptr].actions();

                for action in 0..self[ptr].num_actions() {
                    let ptr = first_child_ptr + action;

                    if let GameState::Won(n) = self[ptr].state() {
                        max_win_len = n.max(max_win_len);
                    } else {
                        proven_loss = false;
                        break;
                    }
                }

                if proven_loss {
                    self[ptr].set_state(GameState::Lost(max_win_len + 1));
                }
            }
            // nothing to do otherwise
            _ => {}
        }
    }

    pub fn set_root_position(&mut self, new_root: &ChessState) {
        let old_root = self.root.clone();
        self.root = new_root.clone();

        if self.is_empty() {
            return;
        }

        let mut found = false;

        println!("info string searching for subtree");

        let root = self.recurse_find(self.root_node(), &old_root, new_root, 2);

        if !root.is_null() && self[root].has_children() {
            found = true;

            if root != self.root_node() {
                self[self.root_node()].clear();
                self.copy_node_across(root, self.root_node());
            }

            println!("info string found subtree");
        }

        if !found {
            println!("info string no subtree found");
            self.clear_halves();
        }
    }

    fn recurse_find(
        &self,
        start: NodePtr,
        this_board: &ChessState,
        board: &ChessState,
        depth: u8,
    ) -> NodePtr {
        if this_board.board() == board.board() {
            return start;
        }

        if start.is_null() || depth == 0 {
            return NodePtr::NULL;
        }

        let first_child_ptr = self[start].actions();

        if first_child_ptr.is_null() {
            return NodePtr::NULL;
        }

        for action in 0..self[start].num_actions() {
            let mut child_board = this_board.clone();

            let child_ptr = first_child_ptr + action;
            let child = &self[child_ptr];

            child_board.make_move(child.parent_move());

            let found = self.recurse_find(child_ptr, &child_board, board, depth - 1);

            if !found.is_null() {
                return found;
            }
        }

        NodePtr::NULL
    }

    pub fn get_best_child(&self, ptr: NodePtr) -> usize {
        self.get_best_child_by_key(ptr, |n| n.visits() as f32)
    }

    pub fn get_best_child_by_key<F: FnMut(&Node) -> f32>(&self, ptr: NodePtr, mut key: F) -> usize {
        let mut best_child = usize::MAX;
        let mut best_score = f32::NEG_INFINITY;

        let first_child_ptr = self[ptr].actions();

        for action in 0..self[ptr].num_actions() {
            let score = key(&self[first_child_ptr + action]);

            if score > best_score {
                best_score = score;
                best_child = action;
            }
        }

        best_child
    }

    #[cfg(feature = "datagen")]
    pub fn get_best_child_temp(&self, ptr: NodePtr, temp: f32) -> Move {
        use rand::prelude::*;
        use rand_distr::Uniform;

        let node = &self[ptr];
        let child_ptr = node.actions();

        if temp == 0.0 {
            return self[child_ptr + self.get_best_child(ptr)].parent_move();
        }

        let mut rng = rand::thread_rng();
        let dist = Uniform::new(0.0, 1.0);
        let rand = dist.sample(&mut rng);

        let mut total = 0.0;
        let mut distribution = vec![0.0; node.num_actions()];
        let t = 1.0 / f64::from(temp);

        for i in 0..node.num_actions() {
            let child = &self[child_ptr + i];
            distribution[i] = f64::from(child.visits()).powf(t);
            total += distribution[i];
        }

        let mut cumulative = 0.0;

        for (i, weight) in distribution.iter().enumerate() {
            cumulative += weight;

            if cumulative / total > rand {
                return self[child_ptr + i].parent_move();
            }
        }

        self[child_ptr + (node.num_actions() - 1)].parent_move()
    }

    #[cfg(feature = "datagen")]
    pub fn add_dirichlet_noise_to_node(&self, ptr: NodePtr, alpha: f32, prop: f32) {
        use rand::prelude::*;
        use rand_distr::Dirichlet;

        let node = &self[ptr];

        if node.num_actions() <= 1 {
            return;
        }

        let actions_ptr = node.actions();

        let mut rng = rand::thread_rng();
        let dist = Dirichlet::new(&vec![alpha; node.num_actions()]).unwrap();
        let samples = dist.sample(&mut rng);

        for (action, &noise) in samples.iter().enumerate() {
            let child = &self[actions_ptr + action];
            let policy = (1.0 - prop) * child.policy() + prop * noise;
            child.set_policy(policy);
        }
    }
}
