use super::init;

pub struct Side;
impl Side {
    pub const WHITE: usize = 0;
    pub const BLACK: usize = 1;
}

pub struct Piece;
impl Piece {
    pub const EMPTY: usize = 0;
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
    pub const DBL: u8 = 1;
    pub const KS: u8 = 2;
    pub const QS: u8 = 3;
    pub const CAP: u8 = 4;
    pub const ENP: u8 = 5;
    pub const NPR: u8 = 8;
    pub const BPR: u8 = 9;
    pub const RPR: u8 = 10;
    pub const QPR: u8 = 11;
    pub const NPC: u8 = 12;
    pub const BPC: u8 = 13;
    pub const RPC: u8 = 14;
    pub const QPC: u8 = 15;
}

// castle rights
pub struct Right;
impl Right {
    pub const WQS: u8 = 0b1000;
    pub const WKS: u8 = 0b0100;
    pub const BQS: u8 = 0b0010;
    pub const BKS: u8 = 0b0001;
    pub const TABLE: [[u8; 2]; 2] = [[Self::WQS, Self::WKS], [Self::BQS, Self::BKS]];
}

// paths required to be clear for castling
pub struct Path;
impl Path {
    pub const BD1: u64 = 0x0000_0000_0000_000E;
    pub const FG1: u64 = 0x0000_0000_0000_0060;
    pub const BD8: u64 = 0x0E00_0000_0000_0000;
    pub const FG8: u64 = 0x6000_0000_0000_0000;
    pub const TABLE: [[u64; 2]; 2] = [[Self::BD1, Self::FG1], [Self::BD8, Self::FG8]];
}

// the castling rook move bitboards
pub const ROOK_MOVES: [[u64; 2]; 2] = [
    [0x0000_0000_0000_0009, 0x0900_0000_0000_0000],
    [0x0000_0000_0000_00A0, 0xA000_0000_0000_0000],
];

// mask off castling rights by square
pub const CASTLE_MASK: [u8; 64] = init! {idx,
    match idx {
         0 =>  7,
         4 =>  3,
         7 => 11,
        56 => 13,
        60 => 12,
        63 => 14,
         _ => 15
    }
};

// for promotions / double pushes
pub struct Rank;
impl Rank {
    pub const PEN: [u64; 2] = [0x00FF_0000_0000_0000, 0x0000_0000_0000_FF00];
    pub const DBL: [u64; 2] = [0x0000_0000_FF00_0000, 0x0000_00FF_0000_0000];
}

pub const IN_BETWEEN: [[u64; 64]; 64] = {
    let mut arr = [[0; 64]; 64];
    let mut i = 0;
    while i < 64 {
        let mut j = 0;
        while j < 64 {
            arr[i][j] = in_between(i, j);
            j += 1;
        }
        i += 1;
    }
    arr
};

pub const LINE_THROUGH: [[u64; 64]; 64] = {
    let mut arr = [[0; 64]; 64];
    let mut i = 0;
    while i < 64 {
        let mut j = 0;
        while j < 64 {
            arr[i][j] = crate::attacks::line_through(i, j);
            j += 1;
        }
        i += 1;
    }
    arr
};

const fn in_between(sq1: usize, sq2: usize) -> u64 {
    const M1: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    const A2A7: u64 = 0x0001_0101_0101_0100;
    const B2G7: u64 = 0x0040_2010_0804_0200;
    const H1B7: u64 = 0x0002_0408_1020_4080;
    let btwn = (M1 << sq1) ^ (M1 << sq2);
    let file = ((sq2 & 7).wrapping_add((sq1 & 7).wrapping_neg())) as u64;
    let rank = (((sq2 | 7).wrapping_sub(sq1)) >> 3) as u64;
    let mut line = ((file & 7).wrapping_sub(1)) & A2A7;
    line += 2 * ((rank & 7).wrapping_sub(1) >> 58);
    line += ((rank.wrapping_sub(file) & 15).wrapping_sub(1)) & B2G7;
    line += ((rank.wrapping_add(file) & 15).wrapping_sub(1)) & H1B7;
    line = line.wrapping_mul(btwn & btwn.wrapping_neg());
    line & btwn
}
