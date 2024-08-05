use std::sync::atomic::{AtomicI16, AtomicU16, AtomicU32, Ordering};

use super::{ActionStats, NodePtr};

#[derive(Debug)]
pub struct Edge {
    ptr: AtomicU32,
    mov: AtomicU16,
    policy: AtomicI16,
    stats: ActionStats,
}

impl Clone for Edge {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicU32::new(self.ptr().inner()),
            mov: AtomicU16::new(self.mov()),
            policy: AtomicI16::new(self.policy.load(Ordering::Relaxed)),
            stats: self.stats.clone(),
        }
    }
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            ptr: AtomicU32::new(NodePtr::NULL.inner()),
            mov: AtomicU16::new(0),
            policy: AtomicI16::new(0),
            stats: ActionStats::default(),
        }
    }
}

impl Edge {
    pub fn new(ptr: NodePtr, mov: u16, policy: i16) -> Self {
        Self {
            ptr: AtomicU32::new(ptr.inner()),
            mov: AtomicU16::new(mov),
            policy: AtomicI16::new(policy),
            stats: ActionStats::default(),
        }
    }

    pub fn set_new(&self, mov: u16, policy: f32) {
        self.ptr.store(NodePtr::NULL.inner(), Ordering::Relaxed);
        self.mov.store(mov, Ordering::Relaxed);
        self.set_policy(policy);
        self.stats.clear();
    }

    pub fn ptr(&self) -> NodePtr {
        NodePtr::from_raw(self.ptr.load(Ordering::Relaxed))
    }

    pub fn mov(&self) -> u16 {
        self.mov.load(Ordering::Relaxed)
    }

    pub fn policy(&self) -> f32 {
        f32::from(self.policy.load(Ordering::Relaxed)) / f32::from(i16::MAX)
    }

    pub fn stats(&self) -> ActionStats {
        self.stats.clone()
    }

    pub fn visits(&self) -> i32 {
        self.stats.visits()
    }

    pub fn q(&self) -> f32 {
        self.stats.q()
    }

    pub fn sq_q(&self) -> f64 {
        self.stats.sq_q()
    }

    pub fn set_ptr(&self, ptr: NodePtr) {
        self.ptr.store(ptr.inner(), Ordering::Relaxed);
    }

    pub fn set_policy(&self, policy: f32) {
        self.policy
            .store((policy * f32::from(i16::MAX)) as i16, Ordering::Relaxed)
    }

    pub fn update(&self, result: f32) {
        self.stats.update(result);
    }
}
