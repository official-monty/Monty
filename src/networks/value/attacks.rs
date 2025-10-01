use montyformat::chess::attacks::{File, DIAGS};

use crate::{init, init_add_assign};

pub struct ValueOffsets;
impl ValueOffsets {
    pub const PAWN: usize = 0;
    pub const KNIGHT: usize = Self::PAWN + 6 * ValueIndices::PAWN;
    pub const BISHOP: usize = Self::KNIGHT + 12 * ValueIndices::KNIGHT[64];
    pub const ROOK: usize = Self::BISHOP + 10 * ValueIndices::BISHOP[64];
    pub const QUEEN: usize = Self::ROOK + 10 * ValueIndices::ROOK[64];
    pub const KING: usize = Self::QUEEN + 12 * ValueIndices::QUEEN[64];
    pub const END: usize = Self::KING + 8 * ValueIndices::KING[64];
}

pub struct ValueIndices;
impl ValueIndices {
    pub const PAWN: usize = 84;
    pub const KNIGHT: [usize; 65] =
        init_add_assign!(|sq, 0, 64| ValueAttacks::KNIGHT[sq].count_ones() as usize);
    pub const BISHOP: [usize; 65] =
        init_add_assign!(|sq, 0, 64| ValueAttacks::BISHOP[sq].count_ones() as usize);
    pub const ROOK: [usize; 65] =
        init_add_assign!(|sq, 0, 64| ValueAttacks::ROOK[sq].count_ones() as usize);
    pub const QUEEN: [usize; 65] =
        init_add_assign!(|sq, 0, 64| ValueAttacks::QUEEN[sq].count_ones() as usize);
    pub const KING: [usize; 65] =
        init_add_assign!(|sq, 0, 64| ValueAttacks::KING[sq].count_ones() as usize);
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
