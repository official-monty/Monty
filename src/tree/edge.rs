use std::sync::atomic::{AtomicI32, AtomicU16, AtomicU32, AtomicI16, Ordering};

#[derive(Debug)]
pub struct Edge {
    ptr: AtomicI32,
    mov: AtomicU16,
    policy: AtomicI16,
    visits: AtomicI32,
    q: AtomicU32,
    sq_q: AtomicU32,
}

impl Clone for Edge {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicI32::new(self.ptr()),
            mov: AtomicU16::new(self.mov()),
            policy: AtomicI16::new(self.policy.load(Ordering::SeqCst)),
            visits: AtomicI32::new(self.visits()),
            q: AtomicU32::new(self.q.load(Ordering::SeqCst)),
            sq_q: AtomicU32::new(self.sq_q.load(Ordering::SeqCst)),
        }
    }
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            ptr: AtomicI32::new(-1),
            mov: AtomicU16::new(0),
            policy: AtomicI16::new(0),
            visits: AtomicI32::new(0),
            q: AtomicU32::new(0),
            sq_q: AtomicU32::new(0),
        }
    }
}

impl Edge {
    pub fn new(ptr: i32, mov: u16, policy: i16) -> Self {
        Self {
            ptr: AtomicI32::new(ptr),
            mov: AtomicU16::new(mov),
            policy: AtomicI16::new(policy),
            visits: AtomicI32::new(0),
            q: AtomicU32::new(0),
            sq_q: AtomicU32::new(0),
        }
    }

    pub fn ptr(&self) -> i32 {
        self.ptr.load(Ordering::SeqCst)
    }

    pub fn mov(&self) -> u16 {
        self.mov.load(Ordering::SeqCst)
    }

    pub fn visits(&self) -> i32 {
        self.visits.load(Ordering::SeqCst)
    }

    pub fn policy(&self) -> f32 {
        f32::from(self.policy.load(Ordering::SeqCst)) / f32::from(i16::MAX)
    }

    fn q64(&self) -> f64 {
        f64::from(self.q.load(Ordering::SeqCst)) / f64::from(u32::MAX)
    }

    pub fn q(&self) -> f32 {
        self.q64() as f32
    }

    pub fn sq_q(&self) -> f64 {
        f64::from(self.sq_q.load(Ordering::SeqCst)) / f64::from(u32::MAX)
    }

    pub fn var(&self) -> f32 {
        (self.sq_q() - self.q64().powi(2)).max(0.0) as f32
    }

    pub fn set_ptr(&mut self, ptr: i32) {
        self.ptr.store(ptr, Ordering::SeqCst);
    }

    pub fn set_policy(&mut self, policy: f32) {
        self.policy.store((policy * f32::from(i16::MAX)) as i16, Ordering::SeqCst)
    }

    pub fn update(&mut self, result: f32) {
        let r = f64::from(result);
        let v = f64::from(self.visits());

        let q = (self.q64() * v + r) / (v + 1.0);
        let sq_q = (self.sq_q() * v + r.powi(2)) / (v + 1.0);

        self.q.store((q * f64::from(u32::MAX)) as u32, Ordering::SeqCst);
        self.sq_q.store((sq_q * f64::from(u32::MAX)) as u32, Ordering::SeqCst);

        self.visits.fetch_add(1, Ordering::SeqCst);
    }
}
