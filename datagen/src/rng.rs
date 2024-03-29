use std::time::{SystemTime, UNIX_EPOCH};

pub struct Rand(u32);

impl Default for Rand {
    fn default() -> Self {
        Self(
            (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("valid")
                .as_nanos()
                & 0xFFFF_FFFF) as u32,
        )
    }
}

impl Rand {
    pub fn rand_int(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    pub fn rand_f32(&mut self, abs_max: f32) -> f32 {
        let rand_int = self.rand_int();
        let float = f64::from(rand_int) / f64::from(u32::MAX);
        (2.0 * float - 1.0) as f32 * abs_max
    }

    pub fn with_seed() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Guaranteed increasing.")
            .as_micros() as u32;

        Self(seed)
    }
}