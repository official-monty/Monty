use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

#[derive(Debug)]
pub struct ActionStats {
    visits: AtomicI32,
    q: AtomicU32,
    sq_q: AtomicU32,
}

impl Clone for ActionStats {
    fn clone(&self) -> Self {
        Self {
            visits: AtomicI32::new(self.visits()),
            q: AtomicU32::new(self.q.load(Ordering::Relaxed)),
            sq_q: AtomicU32::new(self.sq_q.load(Ordering::Relaxed)),
        }
    }
}

impl Default for ActionStats {
    fn default() -> Self {
        Self {
            visits: AtomicI32::new(0),
            q: AtomicU32::new(0),
            sq_q: AtomicU32::new(0),
        }
    }
}

impl ActionStats {
    pub fn visits(&self) -> i32 {
        self.visits.load(Ordering::Relaxed)
    }

    fn q64(&self) -> f64 {
        f64::from(self.q.load(Ordering::Relaxed)) / f64::from(u32::MAX)
    }

    pub fn q(&self) -> f32 {
        self.q64() as f32
    }

    pub fn sq_q(&self) -> f64 {
        f64::from(self.sq_q.load(Ordering::Relaxed)) / f64::from(u32::MAX)
    }

    pub fn var(&self) -> f32 {
        (self.sq_q() - self.q64().powi(2)).max(0.0) as f32
    }

    pub fn update(&self, result: f32) {
        let r = f64::from(result);
        let v = f64::from(self.visits.fetch_add(1, Ordering::Relaxed));

        let q = (self.q64() * v + r) / (v + 1.0);
        let sq_q = (self.sq_q() * v + r.powi(2)) / (v + 1.0);

        self.q.store((q * f64::from(u32::MAX)) as u32, Ordering::Relaxed);
        self.sq_q.store((sq_q * f64::from(u32::MAX)) as u32, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.visits.store(0, Ordering::Relaxed);
        self.q.store(0, Ordering::Relaxed);
        self.sq_q.store(0, Ordering::Relaxed);
    }
}