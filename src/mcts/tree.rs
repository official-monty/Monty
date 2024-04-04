use crate::game::{GameRep, GameState};

#[derive(Default)]
pub struct Tree<T: GameRep> {
    tree: Vec<Node<T>>,
}

impl<T: GameRep> std::ops::Index<i32> for Tree<T> {
    type Output = Node<T>;

    fn index(&self, index: i32) -> &Self::Output {
        &self.tree[index as usize]
    }
}

impl<T: GameRep> std::ops::IndexMut<i32> for Tree<T> {
    fn index_mut(&mut self, index: i32) -> &mut Self::Output {
        &mut self.tree[index as usize]
    }
}

impl<T: GameRep> Tree<T> {
    pub fn push(&mut self, node: Node<T>) -> i32 {
        self.tree.push(node);
        self.tree.len() as i32 - 1
    }

    pub fn map_children<F: FnMut(i32, &Node<T>)>(&self, ptr: i32, mut f: F) {
        let mut child_idx = self.tree[ptr as usize].first_child;
        while child_idx != -1 {
            let child = &self.tree[child_idx as usize];

            f(child_idx, child);

            child_idx = child.next_sibling;
        }
    }

    pub fn map_children_mut<F: FnMut(i32, &mut Node<T>)>(&mut self, ptr: i32, mut f: F) {
        let mut child_idx = self.tree[ptr as usize].first_child;
        while child_idx != -1 {
            let child = &mut self.tree[child_idx as usize];

            f(child_idx, child);

            child_idx = child.next_sibling;
        }
    }

    pub fn expand(&mut self, ptr: i32, pos: &T) {
        let feats = pos.get_policy_feats();
        let mut next_sibling = -1;
        let mut max = f32::NEG_INFINITY;

        pos.map_legal_moves(|mov| {
            let node = Node {
                mov,
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

        self.map_children_mut(ptr, |_, child| {
            child.policy /= total;
        });
    }

    pub fn recurse_find(&self, start: i32, this_board: &T, board: &T, depth: u8) -> i32 {
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

            child_board.make_move(child.mov());

            let found = self.recurse_find(child_idx, &child_board, board, depth - 1);

            if found != -1 {
                return found;
            }

            child_idx = child.next_sibling;
        }

        -1
    }

    pub fn construct_subtree(&self, node_ptr: i32, subtree: &mut Tree<T>) -> i32 {
        let node = &self.tree[node_ptr as usize];
        let new_ptr = subtree.push(node.clone());

        let mut ptr = -1;

        self.map_children(node_ptr, |child_idx, _| {
            let child_ptr = self.construct_subtree(child_idx, subtree);
            subtree[child_ptr].next_sibling = ptr;
            ptr = child_ptr;
        });

        subtree[new_ptr].first_child = ptr;

        new_ptr
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

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.tree.clear();
    }
}

#[derive(Clone)]
pub struct Node<T: GameRep> {
    mov: T::Move,
    state: GameState,
    policy: f32,
    visits: i32,
    wins: f32,
    first_child: i32,
    next_sibling: i32,
}

impl<T: GameRep> Default for Node<T> {
    fn default() -> Self {
        Node {
            mov: T::Move::default(),
            state: GameState::Ongoing,
            policy: 0.0,
            visits: 0,
            wins: 0.0,
            first_child: -1,
            next_sibling: -1,
        }
    }
}

impl<T: GameRep> Node<T> {
    pub fn is_terminal(&self) -> bool {
        self.state != GameState::Ongoing
    }

    pub fn mov(&self) -> T::Move {
        self.mov
    }

    pub fn get_state(&mut self, pos: &T) -> GameState {
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

    pub fn make_root(&mut self) {
        self.state = GameState::Ongoing;
    }
}
