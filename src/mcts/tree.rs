use std::time::Instant;

use crate::{
    game::{GameRep, GameState},
    MctsParams,
};

#[derive(Clone)]
pub struct Node {
    mov: u16,
    mark: Mark,
    state: GameState,
    policy: f32,
    visits: i32,
    wins: f32,
    first_child: i32,
    next_sibling: i32,
}

impl Node {
    pub fn new(mark: Mark) -> Self {
        Node {
            mov: 0,
            mark,
            state: GameState::Ongoing,
            policy: 0.0,
            visits: 0,
            wins: 0.0,
            first_child: -1,
            next_sibling: -1,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.state != GameState::Ongoing
    }

    pub fn mov(&self) -> u16 {
        self.mov
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn policy(&self) -> f32 {
        self.policy
    }

    pub fn visits(&self) -> i32 {
        self.visits
    }

    pub fn q(&self) -> f32 {
        match self.state {
            GameState::Won => 0.0,
            GameState::Lost => 1.0,
            GameState::Draw => 0.5,
            GameState::Ongoing => self.wins / self.visits as f32,
        }
    }

    pub fn has_children(&self) -> bool {
        self.first_child != -1
    }

    pub fn update(&mut self, visits: i32, result: f32) {
        self.visits += visits;
        self.wins += result;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Var1,
    Var2,
    Empty,
}

impl Mark {
    fn flip(&self) -> Self {
        match *self {
            Mark::Empty => Mark::Empty,
            Mark::Var1 => Mark::Var2,
            Mark::Var2 => Mark::Var1,
        }
    }
}

pub struct Tree {
    tree: Vec<Node>,
    root: i32,
    empty: i32,
    used: usize,
    mark: Mark,
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
            tree: vec![Node::new(Mark::Empty); cap],
            root: -1,
            empty: 0,
            used: 0,
            mark: Mark::Empty,
        };

        let end = tree.cap() as i32 - 1;

        for i in 0..end {
            tree[i].first_child = i + 1;
        }

        tree[end].first_child = -1;

        tree
    }

    pub fn push(&mut self, node: Node) -> i32 {
        let new = self.empty;

        assert_ne!(new, -1);

        self.used += 1;
        self.empty = self[self.empty].first_child;
        self[new] = node;

        new
    }

    pub fn delete(&mut self, ptr: i32) {
        self[ptr].mark = Mark::Empty;
        self[ptr].visits = 0;
        self[ptr].first_child = self.empty;
        self.empty = ptr;
        self.used -= 1;
        assert!(self.used < self.cap());
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

        let root = self.root_node();
        self.delete_subtree(root, self[root].mark);
        assert_eq!(self.used, 0);
        assert_eq!(self.empty, root);
        self.root = -1;
        self.mark = Mark::Empty;
    }

    fn delete_subtree(&mut self, ptr: i32, bad_mark: Mark) {
        if self[ptr].mark == bad_mark {
            let mut cptr = self[ptr].first_child;

            while cptr != -1 {
                self.delete_subtree(cptr, bad_mark);
                cptr = self[cptr].next_sibling;
            }

            self.delete(ptr);
        }
    }

    pub fn make_root_node(&mut self, node: i32) {
        self.root = node;
        self.mark = self[node].mark;
        self[node].state = GameState::Ongoing;
    }

    pub fn map_children<F: FnMut(i32, &Node)>(&self, ptr: i32, mut f: F) {
        let mut child_idx = self.tree[ptr as usize].first_child;
        while child_idx != -1 {
            let child = &self.tree[child_idx as usize];

            f(child_idx, child);

            child_idx = child.next_sibling;
        }
    }

    pub fn map_children_mut<F: FnMut(i32, &mut Node)>(&mut self, ptr: i32, mut f: F) {
        let mut child_idx = self.tree[ptr as usize].first_child;
        while child_idx != -1 {
            let child = &mut self.tree[child_idx as usize];

            f(child_idx, child);

            child_idx = child.next_sibling;
        }
    }

    pub fn expand<T: GameRep, const IS_ROOT: bool>(
        &mut self,
        ptr: i32,
        pos: &T,
        params: &MctsParams,
    ) {
        let feats = pos.get_policy_feats();
        let mut next_sibling = -1;
        let mut max = f32::NEG_INFINITY;

        pos.map_legal_moves(|mov| {
            let node = Node {
                mov: mov.into(),
                mark: self.mark,
                state: GameState::Ongoing,
                policy: pos.get_policy(mov, &feats),
                visits: 0,
                wins: 0.0,
                first_child: -1,
                next_sibling,
            };

            if node.policy > max {
                max = node.policy;
            }

            next_sibling = self.push(node);
        });

        let this_node = &mut self.tree[ptr as usize];
        this_node.first_child = next_sibling;

        let mut total = 0.0;

        self.map_children_mut(ptr, |_, child| {
            child.policy = if IS_ROOT {
                ((child.policy - max) / params.root_pst()).exp()
            } else {
                (child.policy - max).exp()
            };
            total += child.policy;
        });

        self.map_children_mut(ptr, |_, child| child.policy /= total);
    }

    pub fn relabel_policy<T: GameRep>(&mut self, ptr: i32, pos: &T, params: &MctsParams) {
        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        self.map_children_mut(ptr, |_, child| {
            child.policy = pos.get_policy(child.mov().into(), &feats);

            if child.policy > max {
                max = child.policy;
            }
        });

        let mut total = 0.0;

        self.map_children_mut(ptr, |_, child| {
            child.policy = ((child.policy - max) / params.root_pst()).exp();
            total += child.policy;
        });

        self.map_children_mut(ptr, |_, child| child.policy /= total);
    }

    pub fn try_use_subtree<T: GameRep>(&mut self, root: &T, prev_board: &Option<T>) {
        let t = Instant::now();

        if self.is_empty() {
            return;
        }

        println!("info string attempting to reuse tree");

        if let Some(board) = prev_board {
            println!("info string searching for subtree");

            let root = self.recurse_find(self.root, board, root, 2);

            if root == -1 || !self[root].has_children() {
                self.clear();
            } else if root != self.root_node() {
                let old_root = self.root_node();
                self.mark_subtree(root);
                self.make_root_node(root);
                self.delete_subtree(old_root, self[old_root].mark);

                println!("info string found subtree of size {} nodes", self.len());
            } else {
                println!(
                    "info string using current tree of size {} nodes",
                    self.len()
                );
            }
        } else {
            self.clear();
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

        let mut child_idx = node.first_child;

        while child_idx != -1 {
            let mut child_board = this_board.clone();
            let child = &self.tree[child_idx as usize];

            child_board.make_move(T::Move::from(child.mov()));

            let found = self.recurse_find(child_idx, &child_board, board, depth - 1);

            if found != -1 {
                return found;
            }

            child_idx = child.next_sibling;
        }

        -1
    }

    fn mark_subtree(&mut self, ptr: i32) {
        self[ptr].mark = self[ptr].mark.flip();

        let mut child = self[ptr].first_child;
        while child != -1 {
            self.mark_subtree(child);
            child = self[child].next_sibling;
        }
    }

    pub fn get_best_child_by_key<F: FnMut(&Node) -> f32>(&self, ptr: i32, mut key: F) -> i32 {
        let mut best_child = -1;
        let mut best_score = f32::NEG_INFINITY;

        self.map_children(ptr, |child_idx, child| {
            let score = key(child);

            if score > best_score {
                best_score = score;
                best_child = child_idx;
            }
        });

        best_child
    }

    pub fn get_best_child(&self, ptr: i32) -> i32 {
        self.get_best_child_by_key(ptr, |child| child.q())
    }

    pub fn display<T: GameRep>(&self, idx: i32, depth: usize) {
        let mut bars = vec![true; depth + 1];
        self.display_recurse::<T>(idx, depth + 1, 0, &mut bars);
    }

    fn display_recurse<T: GameRep>(&self, idx: i32, depth: usize, ply: usize, bars: &mut [bool]) {
        let node = &self[idx];

        if depth == 0 || node.visits == 0 {
            return;
        }

        let mov = if ply > 0 {
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

            T::Move::from(node.mov).to_string()
        } else {
            "root".to_string()
        };

        let mut q = node.q();
        if ply % 2 == 0 {
            q = 1.0 - q;
        }

        println!(
            "{mov} Q({:.2}%) N({}) P({:.2}%) S({})",
            q * 100.0,
            node.visits,
            node.policy * 100.0,
            node.state.to_char(),
        );

        let mut active = Vec::new();
        self.map_children(idx, |child_ptr, child| {
            if child.visits > 0 {
                active.push(child_ptr);
            }
        });

        let end = active.len() - 1;

        for (i, &child_idx) in active.iter().enumerate() {
            if i == end {
                bars[ply] = false;
            }
            self.display_recurse::<T>(child_idx, depth - 1, ply + 1, bars);
            bars[ply] = true;
        }
    }
}
