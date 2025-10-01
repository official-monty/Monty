use std::sync::atomic::{AtomicUsize, Ordering};

use super::{Node, NodePtr};
use crate::chess::GameState;

const CACHE_SIZE: usize = 1024;

pub struct TreeHalf {
    pub(super) nodes: Vec<Node>,
    used: AtomicUsize,
    next: Vec<AtomicUsize>,
    end: Vec<AtomicUsize>,
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
            next: (0..threads).map(|_| AtomicUsize::new(0)).collect(),
            end: (0..threads).map(|_| AtomicUsize::new(0)).collect(),
            half,
        };

        res.nodes.reserve_exact(size);

        unsafe {
            use std::mem::MaybeUninit;
            let chunk_size = size.div_ceil(threads);
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

    pub fn reserve_nodes_thread(&self, num: usize, thread: usize) -> Option<NodePtr> {
        let mut next = self.next[thread].load(Ordering::Relaxed);
        let mut end = self.end[thread].load(Ordering::Relaxed);

        if next + num > end {
            let block = CACHE_SIZE.max(num);
            let start = self.used.fetch_add(block, Ordering::Relaxed);
            if start + block > self.nodes.len() {
                return None;
            }
            next = start;
            end = start + block;
            self.next[thread].store(next + num, Ordering::Relaxed);
            self.end[thread].store(end, Ordering::Relaxed);
            Some(NodePtr::new(self.half, start))
        } else {
            self.next[thread].store(next + num, Ordering::Relaxed);
            Some(NodePtr::new(self.half, next))
        }
    }

    pub fn clear(&self) {
        self.used.store(0, Ordering::Relaxed);
        for (n, e) in self.next.iter().zip(&self.end) {
            n.store(0, Ordering::Relaxed);
            e.store(0, Ordering::Relaxed);
        }
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
