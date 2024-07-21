use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::GameState;
use super::{Node, NodePtr};


pub struct TreeHalf {
    nodes: Vec<Node>,
    used: AtomicUsize,
    half: bool,
    age: AtomicU32,
}

impl std::ops::Index<NodePtr> for TreeHalf {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.nodes[index.idx()]
    }
}

impl TreeHalf {
    pub fn new(size: usize, half: bool) -> Self {
        let mut res = Self {
            nodes: Vec::with_capacity(size),
            used: AtomicUsize::new(0),
            half,
            age: AtomicU32::new(0),
        };

        for _ in 0..size {
            res.nodes.push(Node::new(GameState::Ongoing, 0));
        }

        res
    }

    pub fn push_new(&self, state: GameState) -> NodePtr {
        let idx = self.used.fetch_add(1, Ordering::Relaxed);

        if idx == self.nodes.len() {
            return NodePtr::NULL;
        }

        self.nodes[idx].set_new(state, self.age());

        NodePtr::new(self.half, idx as u32)
    }

    pub fn clear(&self) {
        self.used.store(0, Ordering::Relaxed);
        self.age.fetch_add(1, Ordering::Relaxed);
    }

    pub fn is_empty(&self) -> bool {
        self.used.load(Ordering::Relaxed) == 0
    }

    pub fn is_full(&self) -> bool {
        self.used.load(Ordering::Relaxed) >= self.nodes.len()
    }

    pub fn age(&self) -> u32 {
        self.age.load(Ordering::Relaxed)
    }
}



