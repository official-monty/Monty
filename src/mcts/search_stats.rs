use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;

#[derive(Default, Copy, Clone)]
pub struct ThreadStats {
    pub total_nodes: usize,
    pub total_iters: usize,
    pub main_iters: usize,
    pub seldepth: usize,
}

pub struct SearchStats {
    per_thread: Vec<UnsafeCell<ThreadStats>>, // accessed only by corresponding thread
    pub avg_depth: AtomicUsize,
}

unsafe impl Sync for SearchStats {}

impl SearchStats {
    pub fn new(threads: usize) -> Self {
        Self {
            per_thread: (0..threads)
                .map(|_| UnsafeCell::new(ThreadStats::default()))
                .collect(),
            avg_depth: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn add_iter(&self, tid: usize, depth: usize, main: bool) {
        unsafe {
            let stats = &mut *self.per_thread[tid].get();
            stats.total_iters += 1;
            stats.total_nodes += depth;
            stats.seldepth = stats.seldepth.max(depth.saturating_sub(1));
            if main {
                stats.main_iters += 1;
            }
        }
    }

    pub fn total_iters(&self) -> usize {
        self.per_thread
            .iter()
            .map(|c| unsafe { (*c.get()).total_iters })
            .sum()
    }

    pub fn total_nodes(&self) -> usize {
        self.per_thread
            .iter()
            .map(|c| unsafe { (*c.get()).total_nodes })
            .sum()
    }

    pub fn main_iters(&self) -> usize {
        self.per_thread
            .iter()
            .map(|c| unsafe { (*c.get()).main_iters })
            .sum()
    }

    pub fn seldepth(&self) -> usize {
        self.per_thread
            .iter()
            .map(|c| unsafe { (*c.get()).seldepth })
            .max()
            .unwrap_or(0)
    }
}
