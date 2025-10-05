use montyformat::chess::{
    attacks::{File, DIAGS, KING, KNIGHT},
    consts::{Flag, Piece, Side},
    Move, Position,
};

use crate::networks::policy::see;

pub const NUM_MOVES_INDICES: usize = 2 * FROM_TO;
pub const FROM_TO: usize = OFFSETS[5][64] + PROMOS + 2 + 8;
pub const PROMOS: usize = 4 * 22;

pub fn map_move_to_index(pos: &Position, mov: Move) -> usize {
    let hm = if pos.king_index() % 8 > 3 { 7 } else { 0 };
    let flip = hm ^ if pos.stm() == Side::BLACK { 56 } else { 0 };

    let src = usize::from(mov.src() ^ flip);
    let dst = usize::from(mov.to() ^ flip);

    let good_see = usize::from(see::greater_or_equal_to(pos, &mov, -108));

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

const PAWN: [u64; 64] = init!(|sq, 64| {
    let bit = 1 << sq;
    ((bit & !File::A) << 7) | (bit << 8) | ((bit & !File::H) << 9)
});

const fn bishop(sq: usize) -> u64 {
    let rank = sq / 8;
    let file = sq % 8;

    DIAGS[file + rank].swap_bytes() ^ DIAGS[7 + file - rank]
}

const fn rook(sq: usize) -> u64 {
    let rank = sq / 8;
    let file = sq % 8;

    (0xFF << (rank * 8)) ^ (File::A << file)
}

const fn queen(sq: usize) -> u64 {
    bishop(sq) | rook(sq)
}
