use crate::MoveType;

#[derive(Clone, Copy)]
pub struct Move {
    from: u8,
    to: u8,
    ptr: i32,
    policy: f32,
}

impl Default for Move {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            ptr: -1,
            policy: 0.0,
        }
    }
}

impl MoveType for Move {
    fn is_same_action(self, other: Self) -> bool {
        self.from == other.from && self.to == other.to
    }

    fn policy(&self) -> f32 {
        self.policy
    }

    fn ptr(&self) -> i32 {
        self.ptr
    }

    fn set_policy(&mut self, val: f32) {
        self.policy = val;
    }

    fn set_ptr(&mut self, val: i32) {
        self.ptr = val;
    }
}

impl Move {
    pub fn new_single(to: u8) -> Self {
        Self {
            from: 63,
            to,
            ptr: -1,
            policy: 0.0,
        }
    }

    pub fn new_double(from: u8, to: u8) -> Self {
        Self {
            from,
            to,
            ptr: -1,
            policy: 0.0,
        }
    }

    pub fn new_pass() -> Self {
        Self {
            from: 63,
            to: 63,
            ptr: -1,
            policy: 0.0,
        }
    }

    pub fn is_single(&self) -> bool {
        self.from == 63
    }

    pub fn new_null() -> Self {
        Self {
            from: 0,
            to: 0,
            ptr: -1,
            policy: 0.0,
        }
    }

    #[cfg(feature = "datagen")]
    pub fn is_null(&self) -> bool {
        self.from == 0 && self.to == 0
    }

    pub fn from(&self) -> usize {
        usize::from(self.from)
    }

    pub fn to(&self) -> usize {
        usize::from(self.to)
    }

    pub fn is_pass(&self) -> bool {
        self.to == 63
    }

    pub fn uai(&self) -> String {
        let mut res = String::new();
        let chs = ('a'..'h').collect::<Vec<_>>();

        if self.from() != 63 {
            res += chs[self.from() % 7].to_string().as_str();
            res += format!("{}", 1 + self.from() / 7).as_str()
        }

        if self.to() != 63 {
            res += chs[self.to() % 7].to_string().as_str();
            res += format!("{}", 1 + self.to() / 7).as_str()
        } else {
            res += "0000"
        }

        res
    }
}
