use crate::bitloop;

use super::{consts::Flag, frc::Castling};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Move {
    mov: u16,
}

impl From<Move> for u16 {
    fn from(value: Move) -> Self {
        value.mov
    }
}

impl From<u16> for Move {
    fn from(value: u16) -> Self {
        Self { mov: value }
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uci(&Castling::default()))
    }
}

impl Move {
    pub const NULL: Move = Move { mov: 0 };

    pub fn src(&self) -> u16 {
        self.mov >> 10
    }

    pub fn to(&self) -> u16 {
        (self.mov >> 4) & 63
    }

    pub fn flag(&self) -> u16 {
        self.mov & 15
    }

    pub fn is_capture(&self) -> bool {
        self.flag() & Flag::CAP > 0
    }

    pub fn is_en_passant(&self) -> bool {
        self.flag() == Flag::ENP
    }

    pub fn is_promo(&self) -> bool {
        self.flag() & Flag::NPR > 0
    }

    pub fn promo_pc(&self) -> usize {
        usize::from(self.flag() & 3) + 3
    }

    pub fn new(from: u16, to: u16, flag: u16) -> Self {
        Self {
            mov: (from << 10) | (to << 4) | flag,
        }
    }

    pub fn to_uci(self, castling: &Castling) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) as u8 + b'a') as char, (i / 8) + 1);
        let promo = if self.flag() & 0b1000 > 0 {
            ["n", "b", "r", "q"][(self.flag() & 0b11) as usize]
        } else {
            ""
        };

        let to = if castling.is_chess960() && [Flag::QS, Flag::KS].contains(&self.flag()) {
            let sf = 56 * (self.to() / 56);
            sf + castling.rook_file(usize::from(sf > 0), usize::from(self.flag() == Flag::KS))
        } else {
            self.to()
        };

        format!("{}{}{}", idx_to_sq(self.src()), idx_to_sq(to), promo)
    }
}

#[inline]
pub fn serialise<F: FnMut(Move)>(f: &mut F, attacks: u64, from: u16, flag: u16) {
    bitloop!(|attacks, to| f(Move::new(from, to, flag)));
}
