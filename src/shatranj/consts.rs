use crate::init;

pub struct Side;
impl Side {
    pub const WHITE: usize = 0;
    pub const BLACK: usize = 1;
}

pub struct Piece;
impl Piece {
    pub const PAWN: usize = 2;
    pub const KNIGHT: usize = 3;
    pub const BISHOP: usize = 4;
    pub const ROOK: usize = 5;
    pub const QUEEN: usize = 6;
    pub const KING: usize = 7;
}

pub struct Flag;
impl Flag {
    pub const QUIET: u8 = 0;
    pub const CAP: u8 = 0b10_00_00_00;
    pub const PROMO: u8 = 0b01_00_00_00;
    pub const ALL: u8 = 0b11_00_00_00;
}

pub struct Rank;
impl Rank {
    pub const PEN: [u64; 2] = [0x00FF_0000_0000_0000, 0x0000_0000_0000_FF00];
}

pub struct File;
impl File {
    pub const A: u64 = 0x0101_0101_0101_0101;
    pub const H: u64 = Self::A << 7;
}

const fn rand(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

pub struct ZobristVals {
    pub pcs: [[[u64; 64]; 8]; 2],
    pub c: u64,
}

pub static ZVALS: ZobristVals = {
    let mut seed = 180_620_142;
    seed = rand(seed);

    let c = seed;

    let pcs = init!(|side, 2| init!(|pc, 8| init!(|sq, 64| {
        if pc < 2 {
            0
        } else {
            seed = rand(seed);
            seed
        }
    })));

    ZobristVals { pcs, c }
};
