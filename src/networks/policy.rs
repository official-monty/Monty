use crate::chess::{
    consts::{Flag, Piece, Side},
    Board, Move,
};

use super::{
    accumulator::Accumulator,
    layer::{Layer, TransposedLayer},
};

// DO NOT MOVE
#[allow(non_upper_case_globals, dead_code)]
pub const PolicyFileDefaultName: &str = "nn-06e27b5ef6e7.network";
#[allow(non_upper_case_globals, dead_code)]
pub const CompressedPolicyName: &str = "nn-bef5cb915ecf.network";
#[allow(non_upper_case_globals, dead_code)]
pub const DatagenPolicyFileName: &str = "nn-6764ee301f3e.network";

const QA: i16 = 128;
const QB: i16 = 128;
const FACTOR: i16 = 32;

#[cfg(not(feature = "datagen"))]
pub const L1: usize = 16384;

#[cfg(feature = "datagen")]
pub const L1: usize = 6144;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    l1: Layer<i8, { 768 * 4 }, L1>,
    l2: TransposedLayer<i8, { L1 / 2 }, NUM_MOVES_INDICES>,
}

impl PolicyNetwork {
    pub fn hl(&self, pos: &Board) -> Accumulator<i16, { L1 / 2 }> {
        let mut l1 = Accumulator([0; L1]);

        for (r, &b) in l1.0.iter_mut().zip(self.l1.biases.0.iter()) {
            *r = i16::from(b);
        }

        let mut feats = [0usize; 256];
        let mut count = 0;
        pos.map_features(|feat| {
            feats[count] = feat;
            count += 1;
        });

        l1.add_multi_i8(&feats[..count], &self.l1.weights);

        let mut res = Accumulator([0; L1 / 2]);

        for (elem, (&i, &j)) in res
            .0
            .iter_mut()
            .zip(l1.0.iter().take(L1 / 2).zip(l1.0.iter().skip(L1 / 2)))
        {
            let i = i32::from(i).clamp(0, i32::from(QA));
            let j = i32::from(j).clamp(0, i32::from(QA));
            *elem = ((i * j) / i32::from(QA / FACTOR)) as i16;
        }

        res
    }

    pub fn get(&self, pos: &Board, mov: &Move, hl: &Accumulator<i16, { L1 / 2 }>) -> f32 {
        let idx = map_move_to_index(pos, *mov);
        let weights = &self.l2.weights[idx];

        let mut res = 0;

        for (&w, &v) in weights.0.iter().zip(hl.0.iter()) {
            res += i32::from(w) * i32::from(v);
        }

        (res as f32 / f32::from(QA * FACTOR) + f32::from(self.l2.biases.0[idx])) / f32::from(QB)
    }
}

const NUM_MOVES_INDICES: usize = 2 * FROM_TO;
const FROM_TO: usize = OFFSETS[5][64] + PROMOS + 2 + 8;
const PROMOS: usize = 4 * 22;

pub fn map_move_to_index(pos: &Board, mov: Move) -> usize {
    let hm = if pos.king_index() % 8 > 3 { 7 } else { 0 };
    let flip = hm ^ if pos.stm() == Side::BLACK { 56 } else { 0 };

    let src = usize::from(mov.src() ^ flip);
    let dst = usize::from(mov.to() ^ flip);

    let good_see = usize::from(pos.see(&mov, -108));

    let idx = if mov.is_promo() {
        let ffile = src % 8;
        let tfile = dst % 8;
        let promo_id = 2 * ffile + tfile;

        OFFSETS[5][64] + (PROMOS / 4) * (mov.promo_pc() - Piece::KNIGHT) + promo_id
    } else if mov.flag() == Flag::QS || mov.flag() == Flag::KS {
        let is_ks = usize::from(mov.flag() == Flag::KS);
        let is_hm = usize::from(hm == 0);
        OFFSETS[5][64] + PROMOS + (is_ks ^ is_hm)
    } else if mov.flag() == Flag::DBL {
        OFFSETS[5][64] + PROMOS + 2 + (src % 8)
    } else {
        let pc = pos.get_pc(1 << mov.src()) - 2;
        let below = DESTINATIONS[src][pc] & ((1 << dst) - 1);

        OFFSETS[pc][src] + below.count_ones() as usize
    };

    FROM_TO * good_see + idx
}

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

const OFFSETS: [[usize; 65]; 6] = {
    let mut offsets = [[0; 65]; 6];

    let mut curr = 0;

    let mut pc = 0;
    while pc < 6 {
        let mut sq = 0;

        while sq < 64 {
            offsets[pc][sq] = curr;
            curr += DESTINATIONS[sq][pc].count_ones() as usize;
            sq += 1;
        }

        offsets[pc][64] = curr;

        pc += 1;
    }

    offsets
};

const DESTINATIONS: [[u64; 6]; 64] = init!(|sq, 64| [
    PAWN[sq],
    KNIGHT[sq],
    bishop(sq),
    rook(sq),
    queen(sq),
    KING[sq]
]);

const A: u64 = 0x0101_0101_0101_0101;
const H: u64 = A << 7;

const DIAGS: [u64; 15] = [
    0x0100_0000_0000_0000,
    0x0201_0000_0000_0000,
    0x0402_0100_0000_0000,
    0x0804_0201_0000_0000,
    0x1008_0402_0100_0000,
    0x2010_0804_0201_0000,
    0x4020_1008_0402_0100,
    0x8040_2010_0804_0201,
    0x0080_4020_1008_0402,
    0x0000_8040_2010_0804,
    0x0000_0080_4020_1008,
    0x0000_0000_8040_2010,
    0x0000_0000_0080_4020,
    0x0000_0000_0000_8040,
    0x0000_0000_0000_0080,
];

const PAWN: [u64; 64] = init!(|sq, 64| {
    let bit = 1 << sq;
    ((bit & !A) << 7) | (bit << 8) | ((bit & !H) << 9)
});

const KNIGHT: [u64; 64] = init!(|sq, 64| {
    let n = 1 << sq;
    let h1 = ((n >> 1) & 0x7f7f_7f7f_7f7f_7f7f) | ((n << 1) & 0xfefe_fefe_fefe_fefe);
    let h2 = ((n >> 2) & 0x3f3f_3f3f_3f3f_3f3f) | ((n << 2) & 0xfcfc_fcfc_fcfc_fcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});

const fn bishop(sq: usize) -> u64 {
    let rank = sq / 8;
    let file = sq % 8;

    DIAGS[file + rank].swap_bytes() ^ DIAGS[7 + file - rank]
}

const fn rook(sq: usize) -> u64 {
    let rank = sq / 8;
    let file = sq % 8;

    (0xFF << (rank * 8)) ^ (A << file)
}

const fn queen(sq: usize) -> u64 {
    bishop(sq) | rook(sq)
}

const KING: [u64; 64] = init!(|sq, 64| {
    let mut k = 1 << sq;
    k |= (k << 8) | (k >> 8);
    k |= ((k & !A) >> 1) | ((k & !H) << 1);
    k ^ (1 << sq)
});
