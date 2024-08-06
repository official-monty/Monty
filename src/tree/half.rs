use std::sync::atomic::{AtomicUsize, Ordering};

use super::{Node, NodePtr};
use crate::GameState;

pub struct TreeHalf {
    nodes: Vec<Node>,
    used: AtomicUsize,
    half: bool,
}

impl std::ops::Index<NodePtr> for TreeHalf {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.nodes[index.idx()]
    }
}

impl TreeHalf {
    pub fn new(size: usize, half: bool, threads: usize) -> Self {
        let mut res = Self {
            nodes: Vec::new(),
            used: AtomicUsize::new(0),
            half,
        };

        res.nodes.reserve_exact(size);

        unsafe {
            use std::mem::MaybeUninit;
            let chunk_size = (size + threads - 1) / threads;
            let ptr = res.nodes.as_mut_ptr().cast();
            let uninit: &mut [MaybeUninit<Node>] = std::slice::from_raw_parts_mut(ptr, size);

            std::thread::scope(|s| {
                for chunk in uninit.chunks_mut(chunk_size) {
                    s.spawn(|| {
                        for node in chunk {
                            node.write(Node::new(GameState::Ongoing));
                        }
                    });
                }
            });

            res.nodes.set_len(size);
        }

        res
    }

    pub fn push_new(&self, state: GameState) -> NodePtr {
        let idx = self.used.fetch_add(1, Ordering::Relaxed);

        if idx >= self.nodes.len() {
            return NodePtr::NULL;
        }

        self.nodes[idx].set_new(state);

        NodePtr::new(self.half, idx as u32)
    }

    pub fn clear(&self) {
        self.used.store(0, Ordering::Relaxed);
    }

    pub fn clear_ptrs(&self, threads: usize) {
        if threads == 1 {
            Self::clear_ptrs_single_threaded(self.half, &self.nodes);
        } else {
            self.clear_ptrs_multi_threaded(threads);
        }
    }

    fn clear_ptrs_single_threaded(half: bool, nodes: &[Node]) {
        for node in nodes {
            for action in &mut *node.actions_mut() {
                if action.ptr().half() != half {
                    action.set_ptr(NodePtr::NULL);
                }
            }
        }
    }

    fn clear_ptrs_multi_threaded(&self, threads: usize) {
        std::thread::scope(|s| {
            let chunk_size = (self.nodes.len() + threads - 1) / threads;

            s.spawn(move || {
                for node_chunk in self.nodes.chunks(chunk_size) {
                    Self::clear_ptrs_single_threaded(self.half, node_chunk)
                }
            });

        });
    }

    pub fn is_empty(&self) -> bool {
        self.used.load(Ordering::Relaxed) == 0
    }

    pub fn used(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }

    pub fn is_full(&self) -> bool {
        self.used() >= self.nodes.len()
    }
}
