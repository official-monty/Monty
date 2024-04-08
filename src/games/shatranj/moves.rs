use super::consts::Flag;
use crate::pop_lsb;

#[derive(Copy, Clone, Debug, Default)]
pub struct Move {
    from: u8,
    to_and_flag: u8,
}

impl From<Move> for u16 {
    fn from(value: Move) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<u16> for Move {
    fn from(value: u16) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

impl Move {
    pub const NULL: Move = Move {
        from: 0,
        to_and_flag: 0,
    };

    pub fn from(&self) -> u8 {
        self.from
    }

    pub fn to(&self) -> u8 {
        self.to_and_flag & 63
    }

    pub fn is_quiet(&self) -> bool {
        self.to_and_flag & Flag::ALL == 0
    }

    pub fn is_capture(&self) -> bool {
        self.to_and_flag & Flag::CAP > 0
    }

    pub fn is_promo(&self) -> bool {
        self.to_and_flag & Flag::PROMO > 0
    }

    pub fn new(from: u8, to: u16, flag: u8) -> Self {
        Self {
            from,
            to_and_flag: to as u8 | flag,
        }
    }

    pub fn to_uci(self) -> String {
        let conv = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.is_promo() { "q" } else { "" };

        format!("{}{}{}", conv(self.from()), conv(self.to()), promo)
    }
}

#[inline]
pub fn serialise<F: FnMut(Move)>(f: &mut F, mut attacks: u64, from: u8, flag: u8) {
    while attacks > 0 {
        pop_lsb!(to, attacks);
        f(Move::new(from, to, flag));
    }
}
