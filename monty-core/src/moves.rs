use crate::{consts::Flag, position::Position, PolicyNetwork};

#[macro_export]
macro_rules! pop_lsb {
    ($idx:ident, $x:expr) => {
        let $idx = $x.trailing_zeros() as u8;
        $x &= $x - 1
    };
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Move {
    from: u8,
    to: u8,
    flag: u8,
    moved: u8,
    ptr: i32,
    policy: f32,
}

#[derive(Clone, Default)]
pub struct MoveList {
    list: Vec<Move>,
}

impl Move {
    pub const NULL: Move = Move {
        from: 0,
        to: 0,
        flag: 0,
        moved: 0,
        ptr: -1,
        policy: 0.0,
    };

    pub fn is_same(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.flag == other.flag
            && self.moved == other.moved
    }

    #[must_use]
    pub fn from(&self) -> u8 {
        self.from
    }

    #[must_use]
    pub fn to(&self) -> u8 {
        self.to
    }

    #[must_use]
    pub fn flag(&self) -> u8 {
        self.flag
    }
    #[must_use]
    pub fn moved(&self) -> u8 {
        self.moved
    }

    #[must_use]
    pub fn ptr(&self) -> i32 {
        self.ptr
    }

    #[must_use]
    pub fn policy(&self) -> f32 {
        self.policy
    }

    pub fn set_ptr(&mut self, ptr: i32) {
        self.ptr = ptr;
    }

    pub fn is_capture(&self) -> bool {
        self.flag & Flag::CAP > 0
    }

    pub fn is_en_passant(&self) -> bool {
        self.flag == Flag::ENP
    }

    pub fn is_promo(&self) -> bool {
        self.flag & Flag::NPR > 0
    }

    pub fn promo_pc(&self) -> usize {
        usize::from(self.flag & 3) + 3
    }

    #[must_use]
    pub fn new(from: u8, to: u8, flag: u8, moved: u8) -> Self {
        Self {
            from,
            to,
            flag,
            moved,
            ptr: -1,
            policy: 0.0,
        }
    }

    #[must_use]
    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {
            ["n", "b", "r", "q"][(self.flag & 0b11) as usize]
        } else {
            ""
        };
        format!("{}{}{}", idx_to_sq(self.from), idx_to_sq(self.to), promo)
    }
}

impl std::ops::Deref for MoveList {
    type Target = [Move];
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl std::ops::DerefMut for MoveList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;
    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index]
    }
}

impl std::ops::IndexMut<usize> for MoveList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.list[index]
    }
}

impl MoveList {
    #[inline]
    pub fn push(&mut self, from: u8, to: u8, flag: u8, mpc: usize) {
        self.list.push(Move::new(from, to, flag, mpc as u8));
    }

    #[inline]
    pub fn push_raw(&mut self, mov: Move) {
        self.list.push(mov);
    }

    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        self.list.swap(a, b);
    }

    pub fn set_policies(&mut self, pos: &Position, policy: &PolicyNetwork) {
        let mut total = 0.0;
        let mut max = -1000.0;
        let mut floats = [0.0; 256];
        let feats = pos.get_features();

        for (i, mov) in self.list.iter_mut().enumerate() {
            floats[i] = PolicyNetwork::get(mov, pos, policy, &feats);
            if floats[i] > max {
                max = floats[i];
            }
        }

        for (i, _) in self.list.iter_mut().enumerate() {
            floats[i] = (floats[i] - max).exp();
            total += floats[i];
        }

        for (i, mov) in self.list.iter_mut().enumerate() {
            mov.policy = floats[i] / total;
        }
    }

    #[inline]
    pub fn serialise(&mut self, mut attacks: u64, from: u8, flag: u8, pc: usize) {
        while attacks > 0 {
            pop_lsb!(to, attacks);
            self.push(from, to, flag, pc);
        }
    }
}
