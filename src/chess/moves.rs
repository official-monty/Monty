use super::{consts::Flag, frc::Castling};

#[macro_export]
macro_rules! pop_lsb {
    ($idx:ident, $x:expr) => {
        let $idx = $x.trailing_zeros() as u8;
        $x &= $x - 1
    };
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Move {
    from: u8,
    to: u8,
    flag: u8,
    moved: u8,
}

impl Move {
    pub const NULL: Move = Move {
        from: 0,
        to: 0,
        flag: 0,
        moved: 0,
    };

    pub fn from(&self) -> u8 {
        self.from
    }

    pub fn to(&self) -> u8 {
        self.to
    }

    pub fn flag(&self) -> u8 {
        self.flag
    }

    pub fn moved(&self) -> u8 {
        self.moved
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

    pub fn new(from: u8, to: u8, flag: u8, moved: usize) -> Self {
        Self {
            from,
            to,
            flag,
            moved: moved as u8,
        }
    }

    pub fn to_uci(self, castling: &Castling) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {
            ["n", "b", "r", "q"][(self.flag & 0b11) as usize]
        } else {
            ""
        };

        let to = if castling.is_chess960() && [Flag::QS, Flag::KS].contains(&self.flag) {
            let sf = 56 * (self.to / 56);
            sf + castling.rook_file(usize::from(sf > 0), usize::from(self.flag == Flag::KS))
        } else {
            self.to
        };

        format!("{}{}{}", idx_to_sq(self.from), idx_to_sq(to), promo)
    }
}

#[inline]
pub fn serialise<F: FnMut(Move)>(f: &mut F, mut attacks: u64, from: u8, flag: u8, pc: usize) {
    while attacks > 0 {
        pop_lsb!(to, attacks);
        f(Move::new(from, to, flag, pc));
    }
}
