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

use crate::{
    chess::{ChessState, Move},
    GameState, MctsParams, PolicyNetwork,
};

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
        Self::new(bytes / (48 + 20 * 20), bytes / 48 / 16, threads)
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

    pub fn children_of(&self, ptr: NodePtr) -> &[Node] {
        let node = &self[ptr];
        let ptr = node.actions().inner() as usize;
        let len = node.num_actions();

        &self.tree[usize::from(self.half())].nodes[ptr..ptr + len]
    }

    fn copy_node_across(&self, from: NodePtr, to: NodePtr) -> Option<()> {
        if from == to {
            return Some(());
        }

        let f = &mut *self[from].actions_mut();
        let t = &mut *self[to].actions_mut();

        self[to].set_state(self[from].state());
        self[to].set_gini_impurity(self[from].gini_impurity());

        let num = self[from].num_actions();

        if num == 0 {
            return Some(());
        }

        assert_eq!(self[to].num_actions(), 0);

        *t = *f;
        *f = NodePtr::NULL;

        self[to].set_num_actions(num);

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
    pub fn fetch_node(
        &self,
        parent_ptr: NodePtr,
        action: usize,
    ) -> Option<NodePtr> {
        let first_child_ptr = { *self[parent_ptr].actions() };

        assert!(!first_child_ptr.is_null(), "First child pointer is null, but parent should be expanded!");

        if first_child_ptr.half() != self.half.load(Ordering::Relaxed) {
            let most_recent_ptr = self[parent_ptr].actions_mut();

            if most_recent_ptr.half() == self.half.load(Ordering::Relaxed) {
                return Some(*most_recent_ptr + action);
            }

            assert_eq!(first_child_ptr, *most_recent_ptr);

            let num_children = self[parent_ptr].num_actions();
            let new_ptr = self.tree[self.half()].reserve_nodes(num_children)?;

            self.copy_across(first_child_ptr, num_children, new_ptr);

            Some(new_ptr + action)
        } else {
            Some(first_child_ptr + action)
        }
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

        let mut actions = node.actions_mut();
        let num_actions = node.num_actions();

        if num_actions != 0 {
            return Some(());
        }

        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats, policy);

            // trick for calculating policy before quantising
            actions.push(Edge::new(
                NodePtr::from_raw(f32::to_bits(policy)),
                mov.into(),
                0,
            ));
            max = max.max(policy);
        });

        let pst = match depth {
            0 => unreachable!(),
            1 => params.root_pst(),
            2 => params.depth_2_pst(),
            3.. => 1.0,
        };

        let mut total = 0.0;

        for action in actions.iter_mut() {
            let mut policy = f32::from_bits(action.ptr().inner());

            policy = ((policy - max) / pst).exp();

            action.set_ptr(NodePtr::from_raw(f32::to_bits(policy)));

            total += policy;
        }

        let mut sum_of_squares = 0.0;

        for action in actions.iter_mut() {
            let policy = f32::from_bits(action.ptr().inner()) / total;
            action.set_ptr(NodePtr::NULL);
            action.set_policy(policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        node.set_gini_impurity(gini_impurity);

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
        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        let mut policies = Vec::new();

        for node in self.actions_for(&self[node_ptr]).iter() {
            let mov = Move::from(node.mov());
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

        for (i, action) in self.actions_mut().iter_mut().enumerate() {
            action.set_policy(policies[i] / total);
        }
    }

    pub fn propogate_proven_mates(&self, ptr: NodePtr, child_state: GameState) {
        match child_state {
            // if the child node resulted in a loss, then
            // this node has a guaranteed win
            GameState::Lost(n) => self[ptr].set_state(GameState::Won(n + 1)),
            // if the child node resulted in a win, then check if there are
            // any non-won children, and if not, guaranteed loss for this node
            GameState::Won(n) => {
                let mut proven_loss = true;
                let mut max_win_len = n;
                for action in self[ptr].actions().iter() {
                    if action.ptr().is_null() {
                        proven_loss = false;
                        break;
                    } else if let GameState::Won(n) = self[action.ptr()].state() {
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

    pub fn try_use_subtree(&mut self, root: &ChessState, prev_board: &Option<ChessState>) {
        let t = Instant::now();

        if self.is_empty() {
            self.push_new(GameState::Ongoing).unwrap();
            return;
        }

        println!("info string attempting to reuse tree");

        let mut found = false;

        if let Some(board) = prev_board {
            println!("info string searching for subtree");

            let (root, stats) =
                self.recurse_find(self.root_node(), board, root, self.root_stats.clone(), 2);

            if !root.is_null() && self[root].has_children() {
                found = true;

                if root != self.root_node() {
                    self[self.root_node()].clear();
                    self.copy_across(root, self.root_node());
                    self.root_stats = stats;
                    println!("info string found subtree");
                } else {
                    println!("info string using current tree");
                }
            }
        }

        if !found {
            println!("info string no subtree found");
            self.clear_halves();
            self.push_new(GameState::Ongoing).unwrap();
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
        if this_board.is_same(board) {
            return start;
        }

        if start.is_null() || depth == 0 {
            return NodePtr::NULL;
        }


        for child in self.children_of(start) {
            let mut child_board = this_board.clone();

            child_board.make_move(Move::from(child.mov()));

            let found =
                self.recurse_find(child_idx, &child_board, board, depth - 1);

            if !found.0.is_null() {
                return found;
            }
        }

        NodePtr::NULL
    }

    pub fn get_best_child_by_key<F: FnMut(&Node) -> f32>(&self, ptr: NodePtr, mut key: F) -> usize {
        let mut best_child = usize::MAX;
        let mut best_score = f32::NEG_INFINITY;

        for (i, child) in self.actions_for(ptr).iter().enumerate() {
            let score = key(action);

            if score > best_score {
                best_score = score;
                best_child = i;
            }
        }

        best_child
    }

    pub fn get_best_child(&self, ptr: NodePtr) -> usize {
        self.get_best_child_by_key(ptr, |child| {
            if child.visits() == 0 {
                f32::NEG_INFINITY
            } else if !child.ptr().is_null() {
                match self[child.ptr()].state() {
                    GameState::Lost(n) => 1.0 + f32::from(n),
                    GameState::Won(n) => f32::from(n) - 256.0,
                    GameState::Draw => 0.5,
                    GameState::Ongoing => child.q(),
                }
            } else {
                child.q()
            }
        })
    }
}
