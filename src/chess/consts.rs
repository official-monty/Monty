use super::attacks::{line_through, File, DIAGS};
use crate::{init, init_add_assign};

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

pub const PHASE_VALS: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];

pub const SEE_VALS: [i32; 8] = [0, 0, 100, 450, 450, 650, 1250, 0];

pub struct ValueOffsets;
impl ValueOffsets {
    pub const PAWN_TOTAL: usize = 84;
    pub const KNIGHT: [usize; 65] =
        init_add_assign!(|sq, Self::PAWN_TOTAL, 64| ValueAttacks::KNIGHT[sq].count_ones() as usize);
    pub const BISHOP: [usize; 65] =
        init_add_assign!(|sq, Self::KNIGHT[64], 64| ValueAttacks::KNIGHT[sq].count_ones() as usize);
    pub const ROOK: [usize; 65] =
        init_add_assign!(|sq, Self::BISHOP[64], 64| ValueAttacks::ROOK[sq].count_ones() as usize);
    pub const QUEEN: [usize; 65] =
        init_add_assign!(|sq, Self::ROOK[64], 64| ValueAttacks::QUEEN[sq].count_ones() as usize);
    pub const KING: [usize; 65] =
        init_add_assign!(|sq, Self::QUEEN[64], 64| ValueAttacks::KING[sq].count_ones() as usize);
    pub const END: usize = Self::KING[64];
}

pub struct ValueAttacks;
impl ValueAttacks {
    pub const KNIGHT: [u64; 64] = init!(|sq, 64| {
        let n = 1 << sq;
        let h1 = ((n >> 1) & 0x7f7f_7f7f_7f7f_7f7f) | ((n << 1) & 0xfefe_fefe_fefe_fefe);
        let h2 = ((n >> 2) & 0x3f3f_3f3f_3f3f_3f3f) | ((n << 2) & 0xfcfc_fcfc_fcfc_fcfc);
        (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
    });

    pub const BISHOP: [u64; 64] = init!(|sq, 64| {
        let rank = sq / 8;
        let file = sq % 8;
        DIAGS[file + rank].swap_bytes() ^ DIAGS[7 + file - rank]
    });

    pub const ROOK: [u64; 64] = init!(|sq, 64| {
        let rank = sq / 8;
        let file = sq % 8;
        (0xFF << (rank * 8)) ^ (File::A << file)
    });

    pub const QUEEN: [u64; 64] = init!(|sq, 64| Self::BISHOP[sq] | Self::ROOK[sq]);

    pub const KING: [u64; 64] = init!(|sq, 64| {
        let mut k = 1 << sq;
        k |= (k << 8) | (k >> 8);
        k |= ((k & !File::A) >> 1) | ((k & !File::H) << 1);
        k ^ (1 << sq)
    });
}
