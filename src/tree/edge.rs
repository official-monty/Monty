#[derive(Clone, Copy, Debug)]
pub struct Edge {
    ptr: i32,
    mov: u16,
    policy: i16,
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            ptr: -1,
            mov: 0,
            policy: 0,
        }
    }
}

impl Edge {
    pub fn new(ptr: i32, mov: u16, policy: i16) -> Self {
        Self {
            ptr,
            mov,
            policy,
        }
    }

    pub fn ptr(&self) -> i32 {
        self.ptr
    }

    pub fn mov(&self) -> u16 {
        self.mov
    }

    pub fn policy(&self) -> f32 {
        f32::from(self.policy) / f32::from(i16::MAX)
    }

    pub fn set_ptr(&mut self, ptr: i32) {
        self.ptr = ptr;
    }

    pub fn set_policy(&mut self, policy: f32) {
        self.policy = (policy * f32::from(i16::MAX)) as i16
    }
}
