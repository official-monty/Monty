use crate::{attacks::Attacks, policy::get_policy, params::TunableParams, position::Position, consts::{Side, Piece}};

#[macro_export]
macro_rules! pop_lsb {
    ($idx:ident, $x:expr) => {
        let $idx = $x.trailing_zeros() as u8;
        $x &= $x - 1
    };
}

#[derive(Copy, Clone, Default)]
pub struct Move {
    from: u8,
    to: u8,
    flag: u8,
    moved: u8,
    ptr: i32,
    policy: f64,
}

#[derive(Default)]
pub struct MoveList {
    list: Vec<Move>,
}

impl Move {
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
    pub fn policy(&self) -> f64 {
        self.policy
    }

    pub fn set_ptr(&mut self, ptr: i32) {
        self.ptr = ptr;
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

    pub fn set_policies(&mut self, pos: &Position, params: &TunableParams) {
        let threats = {
            let pawns = pos.opps() & pos.piece(Piece::PAWN);

            if pos.stm() == Side::BLACK {
                Attacks::white_pawn_setwise(pawns)
            } else {
                Attacks::black_pawn_setwise(pawns)
            }
        };
        let mut total = 0.0;

        for mov in self.list.iter_mut() {
            let val = get_policy(mov, threats, params);

            mov.policy = val.exp();
            total += mov.policy;
        }

        for mov in self.list.iter_mut() {
            mov.policy /= total;
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
