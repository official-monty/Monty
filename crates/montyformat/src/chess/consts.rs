use super::attacks::line_through;

macro_rules! init {
    (|$sq:ident, $size:literal | $($rest:tt)+) => {{
        let mut $sq = 0;
        let mut res = [{$($rest)+}; $size];
        while $sq < $size {
            res[$sq] = {$($rest)+};
            $sq += 1;
        }
        res
    }};
}

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
    pub const QUIET: u16 = 0;
    pub const DBL: u16 = 1;
    pub const KS: u16 = 2;
    pub const QS: u16 = 3;
    pub const CAP: u16 = 4;
    pub const ENP: u16 = 5;
    pub const NPR: u16 = 8;
    pub const BPR: u16 = 9;
    pub const RPR: u16 = 10;
    pub const QPR: u16 = 11;
    pub const NPC: u16 = 12;
    pub const BPC: u16 = 13;
    pub const RPC: u16 = 14;
    pub const QPC: u16 = 15;
}

// castle rights
pub struct Right;
impl Right {
    pub const WQS: u8 = 0b1000;
    pub const WKS: u8 = 0b0100;
    pub const BQS: u8 = 0b0010;
    pub const BKS: u8 = 0b0001;
}

// for promotions / double pushes
pub struct Rank;
impl Rank {
    pub const PEN: [u64; 2] = [0x00FF_0000_0000_0000, 0x0000_0000_0000_FF00];
    pub const DBL: [u64; 2] = [0x0000_0000_FF00_0000, 0x0000_00FF_0000_0000];
}

pub static IN_BETWEEN: [[u64; 64]; 64] = {
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

pub static LINE_THROUGH: [[u64; 64]; 64] = {
    let mut arr = [[0; 64]; 64];
    let mut i = 0;
    while i < 64 {
        let mut j = 0;
        while j < 64 {
            arr[i][j] = line_through(i, j);
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

const fn rand(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

pub struct ZobristVals {
    pub pcs: [[[u64; 64]; 8]; 2],
    pub cr: [u64; 16],
    pub enp: [u64; 8],
    pub c: [u64; 2],
}

pub static ZVALS: ZobristVals = {
    let mut seed = 180_620_142;
    seed = rand(seed);

    let c = [0, seed];

    let pcs = init!(|side, 2| init!(|pc, 8| init!(|sq, 64| {
        if pc < 2 {
            0
        } else {
            seed = rand(seed);
            seed
        }
    })));

    let cf = init!(|i, 4| {
        seed = rand(seed);
        seed
    });

    let cr = init!(|i, 16| {
        ((i & 1 > 0) as u64 * cf[0])
            ^ ((i & 2 > 0) as u64 * cf[1])
            ^ ((i & 4 > 0) as u64 * cf[2])
            ^ ((i & 8 > 0) as u64 * cf[3])
    });

    let enp = init!(|i, 8| {
        seed = rand(seed);
        seed
    });

    ZobristVals { pcs, cr, enp, c }
};
