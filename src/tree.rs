mod edge;
mod hash;
mod node;

pub use edge::Edge;
use hash::{HashEntry, HashTable};
pub use node::Node;
use std::time::Instant;

use crate::games::{GameRep, GameState};

pub struct Tree {
    tree: Vec<Node>,
    hash: HashTable,
    root: i32,
    empty: i32,
    used: usize,
    lru_head: i32,
    lru_tail: i32,
    parent_edge: Edge,
}

impl std::ops::Index<i32> for Tree {
    type Output = Node;

    fn index(&self, index: i32) -> &Self::Output {
        &self.tree[index as usize]
    }
}

impl std::ops::IndexMut<i32> for Tree {
    fn index_mut(&mut self, index: i32) -> &mut Self::Output {
        &mut self.tree[index as usize]
    }
}

impl Tree {
    pub fn new_mb(mb: usize) -> Self {
        let cap = mb * 1024 * 1024 / std::mem::size_of::<Node>();
        Self::new(cap)
    }

    fn new(cap: usize) -> Self {
        let mut tree = Self {
            tree: vec![Node::new(GameState::Ongoing, 0, -1, 0); cap / 8],
            hash: HashTable::new(cap / 16),
            root: -1,
            empty: 0,
            used: 0,
            lru_head: -1,
            lru_tail: -1,
            parent_edge: Edge::new(0, 0, 0),
        };

        let end = tree.cap() as i32 - 1;

        for i in 0..end {
            tree[i].set_fwd_link(i + 1);
        }

        tree[end].set_fwd_link(-1);

        tree
    }

    pub fn push(&mut self, node: Node) -> i32 {
        let mut new = self.empty;

        // tree is full, do some LRU pruning
        if new == -1 {
            new = self.lru_tail;
            let parent = self[new].parent();
            let action = self[new].action();

            self.edge_mut(parent, action).set_ptr(-1);

            self.delete(new);
        }

        assert_ne!(new, -1);

        self.used += 1;
        self.empty = self[self.empty].fwd_link();
        self[new] = node;

        self.append_to_lru(new);

        if self.used == 1 {
            self.lru_tail = new;
        }

        new
    }

    pub fn probe_hash(&self, hash: u64) -> Option<HashEntry> {
        self.hash.get(hash)
    }

    pub fn push_hash(&mut self, hash: u64, visits: i32, wins: f32) {
        self.hash.push(hash, visits, wins);
    }

    pub fn delete(&mut self, ptr: i32) {
        self.remove_from_lru(ptr);
        self[ptr].clear();

        let empty = self.empty;
        self[ptr].set_fwd_link(empty);

        self.empty = ptr;
        self.used -= 1;
        assert!(self.used < self.cap());
    }

    pub fn make_recently_used(&mut self, ptr: i32) {
        self.remove_from_lru(ptr);
        self.append_to_lru(ptr);
    }

    fn append_to_lru(&mut self, ptr: i32) {
        let old_head = self.lru_head;
        if old_head != -1 {
            self[old_head].set_bwd_link(ptr);
        }
        self.lru_head = ptr;
        self[ptr].set_fwd_link(old_head);
        self[ptr].set_bwd_link(-1);
    }

    fn remove_from_lru(&mut self, ptr: i32) {
        let bwd = self[ptr].bwd_link();
        let fwd = self[ptr].fwd_link();

        if bwd != -1 {
            self[bwd].set_fwd_link(fwd);
        } else {
            self.lru_head = fwd;
        }

        if fwd != -1 {
            self[fwd].set_bwd_link(bwd);
        } else {
            self.lru_tail = bwd;
        }

        self[ptr].set_bwd_link(-1);
        self[ptr].set_fwd_link(-1);
    }

    pub fn root_node(&self) -> i32 {
        self.root
    }

    pub fn cap(&self) -> usize {
        self.tree.len()
    }

    pub fn len(&self) -> usize {
        self.used
    }

    pub fn remaining(&self) -> usize {
        self.cap() - self.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        if self.used == 0 {
            return;
        }

        self.hash.clear();
        self.root = -1;
        self.empty = 0;
        self.used = 0;
        self.lru_head = -1;
        self.lru_tail = -1;
        self.parent_edge = Edge::new(0, 0, 0);

        let end = self.cap() as i32 - 1;

        for i in 0..end {
            self[i] = Node::new(GameState::Ongoing, 0, -1, 0);
            self[i].set_fwd_link(i + 1);
        }

        self[end].set_fwd_link(-1);
    }

    pub fn make_root_node(&mut self, node: i32) {
        self.root = node;
        self.parent_edge = *self.edge(self[node].parent(), self[node].action());
        self[node].clear_parent();
        self[node].set_state(GameState::Ongoing);
    }

    pub fn edge(&self, ptr: i32, idx: usize) -> &Edge {
        if ptr == -1 {
            &self.parent_edge
        } else {
            &self[ptr].actions()[idx]
        }
    }

    pub fn edge_mut(&mut self, ptr: i32, idx: usize) -> &mut Edge {
        if ptr == -1 {
            &mut self.parent_edge
        } else {
            &mut self[ptr].actions_mut()[idx]
        }
    }

    pub fn propogate_proven_mates(&mut self, ptr: i32, child_state: GameState) {
        match child_state {
            // if the child node resulted in a loss, then
            // this node has a guaranteed win
            GameState::Lost(n) => self[ptr].set_state(GameState::Won(n + 1)),
            // if the child node resulted in a win, then check if there are
            // any non-won children, and if not, guaranteed loss for this node
            GameState::Won(n) => {
                let mut proven_loss = true;
                let mut max_win_len = n;
                for action in self[ptr].actions() {
                    if action.ptr() == -1 {
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

    pub fn try_use_subtree<T: GameRep>(&mut self, root: &T, prev_board: &Option<T>) {
        let t = Instant::now();

        if self.is_empty() {
            let node = self.push(Node::new(GameState::Ongoing, root.hash(), -1, 0));
            self.make_root_node(node);

            return;
        }

        println!("info string attempting to reuse tree");

        let mut found = false;

        if let Some(board) = prev_board {
            println!("info string searching for subtree");

            let root = self.recurse_find(self.root, board, root, 2);

            if root != -1 && self[root].has_children() {
                found = true;

                if root != self.root_node() {
                    self.make_root_node(root);
                    println!("info string found subtree");
                } else {
                    println!("info string using current tree");
                }
            }
        }

        if !found {
            println!("info string no subtree found");
            let node = self.push(Node::new(GameState::Ongoing, root.hash(), -1, 0));
            self.make_root_node(node);
        }

        println!(
            "info string tree processing took {} microseconds",
            t.elapsed().as_micros()
        );
    }

    fn recurse_find<T: GameRep>(&self, start: i32, this_board: &T, board: &T, depth: u8) -> i32 {
        if this_board.is_same(board) {
            return start;
        }

        if start == -1 || depth == 0 {
            return -1;
        }

        let node = &self.tree[start as usize];

        for action in node.actions() {
            let child_idx = action.ptr();
            let mut child_board = this_board.clone();

            child_board.make_move(T::Move::from(action.mov()));

            let found = self.recurse_find(child_idx, &child_board, board, depth - 1);

            if found != -1 {
                return found;
            }
        }

        -1
    }

    fn get_best_child_by_key<F: FnMut(&Edge) -> f32>(&self, ptr: i32, mut key: F) -> usize {
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

    pub fn get_best_child(&self, ptr: i32) -> usize {
        self.get_best_child_by_key(ptr, |child| {
            if child.visits() == 0 {
                f32::NEG_INFINITY
            } else if child.ptr() != -1 {
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

    pub fn display<T: GameRep>(&self, idx: i32, depth: usize) {
        let mut bars = vec![true; depth + 1];
        self.display_recurse::<T>(Edge::new(idx, 0, 0), depth + 1, 0, &mut bars);
    }

    fn display_recurse<T: GameRep>(&self, edge: Edge, depth: usize, ply: usize, bars: &mut [bool]) {
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

            let mov = T::Move::from(edge.mov()).to_string();

            println!(
                "{mov} Q({:.2}%) N({}) P({:.2}%) S({})",
                q * 100.0,
                edge.visits(),
                edge.policy() * 100.0,
                node.state(),
            );
        } else {
            println!("root");
        }

        let mut active = Vec::new();
        for &action in node.actions() {
            if action.ptr() != -1 {
                active.push(action);
            }
        }

        let end = active.len() - 1;

        for (i, &action) in active.iter().enumerate() {
            if i == end {
                bars[ply] = false;
            }
            if action.visits() > 0 {
                self.display_recurse::<T>(action, depth - 1, ply + 1, bars);
            }
            bars[ply] = true;
        }
    }
}
