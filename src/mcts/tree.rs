use crate::game::{GameRep, GameState};

pub struct Tree {
    tree: Vec<Node>,
    root: i32,
    empty: i32,
    used: usize,
    mark: bool,
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
            tree: vec![Node::default(); cap],
            root: -1,
            empty: 0,
            used: 0,
            mark: false,
        };

        tree.clear();

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
        self.root = -1;
        self.empty = 0;
        self.used = 0;
        self.mark = false;

        let end = self.cap() as i32 - 1;

        for i in 0..end {
            self[i].visits = 0;
            self[i].mark = false;
            self[i].first_child = i + 1;
        }

        self[end].first_child = -1;
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

    pub fn expand<T: GameRep>(&mut self, ptr: i32, pos: &T) {
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
            child.policy = (child.policy - max).exp();
            total += child.policy;
        });

        self.map_children_mut(ptr, |_, child| child.policy /= total);
    }

    pub fn try_use_subtree<T: GameRep>(&mut self, root: &T, prev_board: &Option<T>) {
        if self.is_empty() {
            return;
        }

        if let Some(board) = prev_board {
            println!("info string searching for subtree");

            let root = self.recurse_find(self.root, board, root, 2);

            if root == -1 || !self[root].has_children() {
                self.clear();
            } else {
                self.mark_subtree(root);
                self.make_root_node(root);
                self.clear_unmarked();

                println!(
                    "info string found subtree of size {} nodes",
                    self.len()
                );
            }
        } else {
            self.clear();
        }
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
        self[ptr].mark = !self[ptr].mark;

        let mut child = self[ptr].first_child;
        while child != -1 {
            self.mark_subtree(child);
            child = self[child].next_sibling;
        }
    }

    fn clear_unmarked(&mut self) {
        let mark = self.mark;

        for i in 0..self.cap() as i32 {
            if self[i].visits > 0 && self[i].mark != mark {
                self.delete(i);
            }
        }
    }

    pub fn get_best_child(&self, ptr: i32) -> i32 {
        let mut best_child = -1;
        let mut best_score = -100.0;

        self.map_children(ptr, |child_idx, child| {
            let score = child.q();

            if score > best_score {
                best_score = score;
                best_child = child_idx;
            }
        });

        best_child
    }
}

#[derive(Clone)]
pub struct Node {
    mov: u16,
    mark: bool,
    state: GameState,
    policy: f32,
    visits: i32,
    wins: f32,
    first_child: i32,
    next_sibling: i32,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            mov: 0,
            mark: false,
            state: GameState::Ongoing,
            policy: 0.0,
            visits: 0,
            wins: 0.0,
            first_child: -1,
            next_sibling: -1,
        }
    }
}

impl Node {
    pub fn is_terminal(&self) -> bool {
        self.state != GameState::Ongoing
    }

    pub fn mov(&self) -> u16 {
        self.mov
    }

    pub fn get_state<T: GameRep>(&mut self, pos: &T) -> GameState {
        self.state = pos.game_state();
        self.state
    }

    pub fn policy(&self) -> f32 {
        self.policy
    }

    pub fn visits(&self) -> i32 {
        self.visits
    }

    pub fn q(&self) -> f32 {
        self.wins / self.visits as f32
    }

    pub fn has_children(&self) -> bool {
        self.first_child != -1
    }

    pub fn update(&mut self, visits: i32, result: f32) {
        self.visits += visits;
        self.wins += result;
    }
}
