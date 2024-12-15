mod half;
mod hash;
mod node;

use half::TreeHalf;
use hash::{HashEntry, HashTable};
pub use node::{Node, NodePtr};

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use crate::{chess::ChessState, mcts::SearchHelpers, GameState, MctsParams, PolicyNetwork};

pub struct Tree {
    tree: [TreeHalf; 2],
    half: AtomicBool,
    hash: HashTable,
}

impl std::ops::Index<NodePtr> for Tree {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.tree[usize::from(index.half())][index]
    }
}

impl Tree {
    pub fn new_mb(mb: usize, threads: usize) -> Self {
        let bytes = mb * 1024 * 1024;

        Self::new(bytes / 48, bytes / 48 / 16, threads)
    }

    fn new(tree_cap: usize, hash_cap: usize, threads: usize) -> Self {
        Self {
            tree: [
                TreeHalf::new(tree_cap / 2, false, threads),
                TreeHalf::new(tree_cap / 2, true, threads),
            ],
            half: AtomicBool::new(false),
            hash: HashTable::new(hash_cap / 4, threads),
        }
    }

    pub fn half(&self) -> usize {
        usize::from(self.half.load(Ordering::Relaxed))
    }

    pub fn is_full(&self) -> bool {
        self.tree[self.half()].is_full()
    }

    pub fn push_new_node(&self) -> Option<NodePtr> {
        self.tree[self.half()].reserve_nodes(1)
    }

    fn copy_node_across(&self, from: NodePtr, to: NodePtr) -> Option<()> {
        if from == to {
            return Some(());
        }

        let f = &mut *self[from].actions_mut();
        let t = &mut *self[to].actions_mut();

        // no other thread is able to modify `from`
        // whilst the above write locks are held,
        // so this will never result in copying garbage
        // (for a thread that calls this function whilst
        // another thread is already doing the same work)
        self[to].copy_from(&self[from]);
        self[to].set_num_actions(self[from].num_actions());
        *t = *f;

        Some(())
    }

    pub fn copy_across(&self, from: NodePtr, num: usize, to: NodePtr) -> Option<()> {
        for i in 0..num {
            self.copy_node_across(from + i, to + i)?;
        }

        Some(())
    }

    pub fn flip(&self, copy_across: bool, threads: usize) {
        let old_root_ptr = self.root_node();

        let old = usize::from(self.half.fetch_xor(true, Ordering::Relaxed));
        self.tree[old].clear_ptrs(threads);
        self.tree[old ^ 1].clear();

        if copy_across {
            let new_root_ptr = self.tree[self.half()].reserve_nodes(1).unwrap();
            self[new_root_ptr].clear();

            self.copy_node_across(old_root_ptr, new_root_ptr);
        }
    }

    #[must_use]
    pub fn fetch_children(&self, parent_ptr: NodePtr) -> Option<()> {
        let first_child_ptr = { *self[parent_ptr].actions() };

        if first_child_ptr.half() != self.half.load(Ordering::Relaxed) {
            let mut most_recent_ptr = self[parent_ptr].actions_mut();

            if most_recent_ptr.half() == self.half.load(Ordering::Relaxed) {
                return Some(());
            }

            assert_eq!(first_child_ptr, *most_recent_ptr);

            let num_children = self[parent_ptr].num_actions();
            let new_ptr = self.tree[self.half()].reserve_nodes(num_children)?;

            self.copy_across(first_child_ptr, num_children, new_ptr);

            *most_recent_ptr = new_ptr;
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
    ) -> Option<()> {
        let node = &self[node_ptr];

        let mut actions_ptr = node.actions_mut();

        // when running with >1 threads, this function may
        // be called twice, and this acts as a safeguard in
        // that case
        if !node.is_not_expanded() {
            return Some(());
        }

        let feats = pos.get_policy_feats(policy);
        let mut max = f32::NEG_INFINITY;
        let mut actions = Vec::new();

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats, policy);
            actions.push((mov, policy));
            max = max.max(policy);
        });

        let new_ptr = self.tree[self.half()].reserve_nodes(actions.len())?;

        let pst = match depth {
            0 => unreachable!(),
            1 => params.root_pst(),
            2 => params.depth_2_pst(),
            3.. => SearchHelpers::get_pst(self[node_ptr].q(), params),
        };

        let mut total = 0.0;

        for (_, policy) in actions.iter_mut() {
            *policy = ((*policy - max) / pst).exp();
            total += *policy;
        }

        let mut sum_of_squares = 0.0;

        for (action, &(mov, policy)) in actions.iter().enumerate() {
            let ptr = new_ptr + action;
            let policy = policy / total;

            self[ptr].set_new(mov, policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        node.set_gini_impurity(gini_impurity);

        *actions_ptr = new_ptr;
        node.set_num_actions(actions.len());

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
        let feats = pos.get_policy_feats(policy);
        let mut max = f32::NEG_INFINITY;

        let mut policies = Vec::new();

        let actions = self[node_ptr].actions_mut();
        let num_actions = self[node_ptr].num_actions();

        for action in 0..num_actions {
            let mov = self[*actions + action].parent_move();
            let policy = pos.get_policy(mov, &feats, policy);

            policies.push(policy);
            max = max.max(policy);
        }

        let pst = match depth {
            0 => unreachable!(),
            1 => params.root_pst(),
            2 => params.depth_2_pst(),
            3.. => unreachable!(),
        };

        let mut total = 0.0;

        for policy in &mut policies {
            *policy = ((*policy - max) / pst).exp();
            total += *policy;
        }

        let mut sum_of_squares = 0.0;

        for (action, &policy) in policies.iter().enumerate() {
            let policy = policy / total;
            self[*actions + action].set_policy(policy);
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
                let first_child_ptr = *self[ptr].actions();

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

    pub fn try_use_subtree(&self, root: &ChessState, prev_board: &Option<ChessState>) {
        let t = Instant::now();

        if self.is_empty() {
            return;
        }

        println!("info string attempting to reuse tree");

        let mut found = false;

        if let Some(board) = prev_board {
            println!("info string searching for subtree");

            let root = self.recurse_find(self.root_node(), board, root, 2);

            if !root.is_null() && self[root].has_children() {
                found = true;

                if root != self.root_node() {
                    self[self.root_node()].clear();
                    self.copy_node_across(root, self.root_node());
                    println!("info string found subtree");
                } else {
                    println!("info string using current tree");
                }
            }
        }

        if !found {
            println!("info string no subtree found");
            self.clear_halves();
        }

        println!(
            "info string tree processing took {} microseconds",
            t.elapsed().as_micros()
        );
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

        let first_child_ptr = { *self[start].actions() };

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

    pub fn get_best_child_by_key<F: FnMut(&Node) -> f32>(&self, ptr: NodePtr, mut key: F) -> usize {
        let mut best_child = usize::MAX;
        let mut best_score = f32::NEG_INFINITY;

        let first_child_ptr = { *self[ptr].actions() };

        for action in 0..self[ptr].num_actions() {
            let score = key(&self[first_child_ptr + action]);

            if score > best_score {
                best_score = score;
                best_child = action;
            }
        }

        best_child
    }

    pub fn get_best_child(&self, ptr: NodePtr) -> usize {
        self.get_best_child_by_key(ptr, |child| {
            if child.visits() == 0 {
                f32::NEG_INFINITY
            } else {
                match child.state() {
                    GameState::Lost(n) => 1.0 + f32::from(n),
                    GameState::Won(n) => f32::from(n) - 256.0,
                    GameState::Draw => 0.5,
                    GameState::Ongoing => child.q(),
                }
            }
        })
    }
}
