mod edge;
mod half;
mod hash;
mod node;
mod ptr;
mod stats;

pub use edge::Edge;
use half::TreeHalf;
use hash::{HashEntry, HashTable};
pub use node::Node;
pub use ptr::NodePtr;
pub use stats::ActionStats;

use std::{
    sync::{atomic::{AtomicBool, Ordering}, Mutex},
    time::Instant,
};

use crate::{
    chess::{ChessState, Move}, GameState
};

pub struct Tree {
    tree: [TreeHalf; 2],
    half: AtomicBool,
    flip_lock: Mutex<()>,
    hash: HashTable,
    root_stats: ActionStats,
}

impl std::ops::Index<NodePtr> for Tree {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.tree[usize::from(index.half())][index]
    }
}

impl Tree {
    pub fn new_mb(mb: usize) -> Self {
        let cap = mb * 1024 * 1024 / 48;
        Self::new(cap)
    }

    fn new(cap: usize) -> Self {
        let tree_size = cap / 8;

        println!("info string tree size {tree_size}");

        Self {
            tree: [
                TreeHalf::new(tree_size, false),
                TreeHalf::new(tree_size, true),
            ],
            half: AtomicBool::new(false),
            hash: HashTable::new(cap / 16),
            root_stats: ActionStats::default(),
            flip_lock: Mutex::new(()),
        }
    }

    pub fn half(&self) -> usize {
        usize::from(self.half.load(Ordering::Relaxed))
    }

    pub fn is_full(&self) -> bool {
        self.tree[self.half()].is_full()
    }

    fn flip(&self) {
        let old = self.half.fetch_xor(true, Ordering::Relaxed);
        let new = usize::from(!old);
        self.tree[new].clear();
    }

    pub fn copy_across(&self, from: NodePtr, to: NodePtr) {
        if from == to {
            return;
        }

        self[to].set_state(self[from].state());

        // need these mut refs to be dropped immediately
        {
            let f = &mut *self[from].actions_mut();
            let t = &mut *self[to].actions_mut();
            std::mem::swap(f, t);
        }

        let half = self.half.load(Ordering::Relaxed);
        let actions = self[to].actions();
        for action in actions.iter() {
            if action.ptr().half() == half {
                action.set_ptr(NodePtr::NULL);
            }
        }
    }

    #[must_use]
    pub fn push_new(&self, state: GameState) -> Option<NodePtr> {
        let new_ptr = self.tree[self.half()].push_new(state);

        if new_ptr.is_null() {
            let _lock = self.flip_lock.lock();

            if self.is_full() {
                let old_root_ptr = self.root_node();

                self.flip();

                let new_root_ptr = self.tree[self.half()].push_new(GameState::Ongoing);
                assert_eq!(new_root_ptr, self.root_node());

                self.copy_across(old_root_ptr, new_root_ptr);
            }

            None
        } else {
            Some(new_ptr)
        }
    }

    #[must_use]
    pub fn fetch_node(
        &self,
        pos: &ChessState,
        parent_ptr: NodePtr,
        ptr: NodePtr,
        action: usize,
    ) -> Option<NodePtr> {
        if ptr.is_null() || self.is_old(ptr) {
            let state = pos.game_state();
            let new_ptr = self.push_new(state)?;
            self.set_edge_ptr(parent_ptr, action, new_ptr);
            Some(new_ptr)
        } else if ptr.half() != self.half.load(Ordering::Relaxed) {
            let new_ptr = self.push_new(GameState::Ongoing)?;
            self.copy_across(ptr, new_ptr);
            self.set_edge_ptr(parent_ptr, action, new_ptr);
            Some(new_ptr)
        } else {
            Some(ptr)
        }
    }

    fn is_old(&self, ptr: NodePtr) -> bool {
        self.tree[usize::from(ptr.half())].age() != self[ptr].age()
    }

    pub fn root_node(&self) -> NodePtr {
        NodePtr::new(self.half.load(Ordering::Relaxed), 0)
    }

    pub fn root_stats(&self) -> &ActionStats {
        &self.root_stats
    }

    pub fn edge_copy(&self, ptr: NodePtr, action: usize) -> Edge {
        self[ptr].actions()[action].clone()
    }

    pub fn set_edge_ptr(&self, ptr: NodePtr, action: usize, set: NodePtr) {
        self[ptr].actions()[action].set_ptr(set);
    }

    pub fn update_edge_stats(&self, ptr: NodePtr, action: usize, result: f32) -> f32 {
        let actions = &self[ptr].actions();
        assert!(actions.len() > action, "node: {ptr:?}");
        let edge = &actions[action];
        edge.update(result);
        edge.q()
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

    pub fn clear(&mut self) {
        self.clear_halves();
        self.hash.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.tree[0].is_empty() && self.tree[1].is_empty()
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
            let node = self.push_new(GameState::Ongoing).unwrap();
            assert_eq!(node, self.root_node());
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
                    self.flip();
                    self.push_new(GameState::Ongoing).unwrap();
                    self.copy_across(root, self.root_node());
                    println!("info string found subtree");
                } else {
                    println!("info string using current tree");
                }
            }
        }

        if !found {
            println!("info string no subtree found");
            self.clear_halves();
            self.flip();
            let node = self.push_new(GameState::Ongoing).unwrap();
            assert_eq!(node, self.root_node());
        }

        println!(
            "info string tree processing took {} microseconds",
            t.elapsed().as_micros()
        );
    }

    pub fn ptr_is_valid(&self, ptr: NodePtr) -> bool {
        ptr.half() == self.half.load(Ordering::Relaxed)
            && self.tree[self.half()].age() == self[ptr].age()
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

        let node = &self[start];

        for action in node.actions().iter() {
            let child_idx = action.ptr();
            let mut child_board = this_board.clone();

            child_board.make_move(Move::from(action.mov()));

            let found = self.recurse_find(child_idx, &child_board, board, depth - 1);

            if !found.is_null() {
                return found;
            }
        }

        NodePtr::NULL
    }

    pub fn get_best_child_by_key<F: FnMut(&Edge) -> f32>(&self, ptr: NodePtr, mut key: F) -> usize {
        let mut best_child = usize::MAX;
        let mut best_score = f32::NEG_INFINITY;

        for (i, action) in self[ptr].actions().iter().enumerate() {
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

    pub fn display(&self, idx: NodePtr, depth: usize) {
        let mut bars = vec![true; depth + 1];
        self.display_recurse(&Edge::new(idx, 0, 0), depth + 1, 0, &mut bars);
    }

    fn display_recurse(&self, edge: &Edge, depth: usize, ply: usize, bars: &mut [bool]) {
        let node = &self[edge.ptr()];

        if depth == 0 {
            return;
        }

        let mut q = edge.q();
        if ply % 2 == 0 {
            q = 1.0 - q;
        }

        if ply > 0 {
            for &bar in bars.iter().take(ply - 1) {
                if bar {
                    print!("\u{2502}   ");
                } else {
                    print!("    ");
                }
            }

            if bars[ply - 1] {
                print!("\u{251C}\u{2500}> ");
            } else {
                print!("\u{2514}\u{2500}> ");
            }

            let mov = Move::from(edge.mov()).to_string();

            println!(
                "{mov} Q({:.2}%) N({}) P({:.2}%) S({})",
                q * 100.0,
                edge.visits(),
                edge.policy() * 100.0,
                node.state(),
            );
        } else {
            println!("root Q({:.2}%)", self.root_stats.q() * 100.0);
        }

        let mut active = Vec::new();
        for action in node.actions().iter() {
            if !action.ptr().is_null() {
                active.push(action.clone());
            }
        }

        let end = active.len() - 1;

        for (i, action) in active.iter().enumerate() {
            if i == end {
                bars[ply] = false;
            }
            if action.visits() > 0 {
                self.display_recurse(action, depth - 1, ply + 1, bars);
            }
            bars[ply] = true;
        }
    }
}
