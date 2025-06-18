use std::sync::atomic::{AtomicUsize, Ordering};

use super::{Node, NodePtr};
use crate::chess::GameState;

pub struct TreeHalf {
    pub(super) nodes: Vec<Node>,
    used: Vec<AtomicUsize>,
    chunk_size: usize,
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
            used: (0..threads).map(|_| AtomicUsize::new(0)).collect(),
            chunk_size: size.div_ceil(threads),
            half,
        };

        res.nodes.reserve_exact(size);

        unsafe {
            use std::mem::MaybeUninit;
            let ptr = res.nodes.as_mut_ptr().cast();
            let uninit: &mut [MaybeUninit<Node>] =
                std::slice::from_raw_parts_mut(ptr, size);

            std::thread::scope(|s| {
                for chunk in uninit.chunks_mut(res.chunk_size) {
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

    pub fn reserve_nodes_thread(&self, num: usize, thread: usize) -> Option<NodePtr> {
        let idx = self.used[thread].fetch_add(num, Ordering::Relaxed);
        if idx + num > self.chunk_size {
            return None;
        }

        Some(NodePtr::new(self.half, (thread * self.chunk_size + idx) as u32))
    }

    pub fn clear(&self) {
        for used in &self.used {
            used.store(0, Ordering::Relaxed);
        }
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
            let actions_half = { node.actions().half() };

            if actions_half != half {
                node.clear_actions();
            }
        }
    }

    fn clear_ptrs_multi_threaded(&self, threads: usize) {
        std::thread::scope(|s| {
            let chunk_size = self.nodes.len().div_ceil(threads);
            let half = self.half;

            for chunk in self.nodes.chunks(chunk_size) {
                s.spawn(move || {
                    Self::clear_ptrs_single_threaded(half, chunk);
                });
            }
        });
    }

    pub fn is_empty(&self) -> bool {
        self.used.iter().all(|u| u.load(Ordering::Relaxed) == 0)
    }

    pub fn used(&self) -> usize {
        self.used.iter().map(|u| u.load(Ordering::Relaxed)).sum()
    }

    pub fn is_full(&self) -> bool {
        self.used() >= self.nodes.len()
    }
}
