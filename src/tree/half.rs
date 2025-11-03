use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Mutex,
};

use super::{Node, NodePtr};
use crate::chess::GameState;

const CACHE_SIZE: usize = 1024;

pub struct TreeHalf {
    pub(super) nodes: Vec<Node>,
    used: AtomicUsize,
    next: Vec<AtomicUsize>,
    end: Vec<AtomicUsize>,
    half: bool,
    cross_links: Mutex<Vec<usize>>,
    cross_link_marks: Vec<AtomicU64>,
    cross_link_epoch: AtomicU64,
}

impl std::ops::Index<NodePtr> for TreeHalf {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.nodes[index.idx()]
    }
}

impl TreeHalf {
    pub fn new(size: usize, half: bool, threads: usize) -> Self {
        let cross_links = Mutex::new(Vec::new());
        let cross_link_marks = (0..size).map(|_| AtomicU64::new(0)).collect();

        let mut res = Self {
            nodes: Vec::new(),
            used: AtomicUsize::new(0),
            next: (0..threads).map(|_| AtomicUsize::new(0)).collect(),
            end: (0..threads).map(|_| AtomicUsize::new(0)).collect(),
            half,
            cross_links,
            cross_link_marks,
            cross_link_epoch: AtomicU64::new(1),
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

        self.cross_link_epoch.fetch_add(1, Ordering::Relaxed);
        self.cross_links.lock().unwrap().clear();
    }

    pub fn clear_cross_links(&self, target_half: bool) {
        let epoch = self.cross_link_epoch.load(Ordering::Relaxed);
        let mut links = self.cross_links.lock().unwrap();
        let mut idx = 0;
        let mut to_clear = Vec::new();

        while idx < links.len() {
            let node_idx = links[idx];
            if self.cross_link_marks[node_idx].load(Ordering::Relaxed) != epoch {
                links.swap_remove(idx);
                continue;
            }

            let node_ptr = NodePtr::new(self.half, node_idx);
            let actions = self[node_ptr].actions();

            if actions.is_null() || actions.half() == self.half {
                self.cross_link_marks[node_idx].store(0, Ordering::Relaxed);
                links.swap_remove(idx);
                continue;
            }

            if actions.half() != target_half {
                idx += 1;
                continue;
            }

            self.cross_link_marks[node_idx].store(0, Ordering::Relaxed);
            to_clear.push(node_ptr);
            links.swap_remove(idx);
        }

        drop(links);

        for node_ptr in to_clear {
            self[node_ptr].clear_actions();
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

    pub fn register_cross_link(&self, node: NodePtr, target: NodePtr) {
        debug_assert_eq!(node.half(), self.half);

        if target.is_null() || target.half() == self.half {
            self.cross_link_marks[node.idx()].store(0, Ordering::Relaxed);
            return;
        }

        let epoch = self.cross_link_epoch.load(Ordering::Relaxed);
        if self.cross_link_marks[node.idx()].swap(epoch, Ordering::Relaxed) != epoch {
            self.cross_links.lock().unwrap().push(node.idx());
        }
    }
}
