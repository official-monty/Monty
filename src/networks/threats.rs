use crate::chess::{
    consts::{Piece, Side, ValueAttacks, ValueIndices, ValueOffsets},
    Attacks, Board,
};

const TOTAL_THREATS: usize = 2 * ValueOffsets::END;
pub const TOTAL: usize = TOTAL_THREATS + 768;

pub fn map_features<F: FnMut(usize)>(pos: &Board, mut f: F) {
    let mut bbs = pos.bbs();

    // flip to stm perspective
    if pos.stm() == Side::BLACK {
        bbs.swap(0, 1);
        for bb in bbs.iter_mut() {
            *bb = bb.swap_bytes()
        }
    }

    // horiontal mirror
    let ksq = (bbs[0] & bbs[Piece::KING]).trailing_zeros();
    if ksq % 8 > 3 {
        for bb in bbs.iter_mut() {
            *bb = flip_horizontal(*bb);
        }
    };

    let mut pieces = [13; 64];
    for side in [Side::WHITE, Side::BLACK] {
        for piece in Piece::PAWN..=Piece::KING {
            let pc = 6 * side + piece - 2;
            map_bb(bbs[side] & bbs[piece], |sq| pieces[sq] = pc);
        }
    }

    let occ = bbs[0] | bbs[1];

    for side in [Side::WHITE, Side::BLACK] {
        let side_offset = ValueOffsets::END * side;
        let opps = bbs[side ^ 1];

        for piece in Piece::PAWN..=Piece::KING {
            map_bb(bbs[side] & bbs[piece], |sq| {
                let threats = match piece {
                    Piece::PAWN => Attacks::pawn(sq, side),
                    Piece::KNIGHT => Attacks::knight(sq),
                    Piece::BISHOP => Attacks::bishop(sq, occ),
                    Piece::ROOK => Attacks::rook(sq, occ),
                    Piece::QUEEN => Attacks::queen(sq, occ),
                    Piece::KING => Attacks::king(sq),
                    _ => unreachable!(),
                } & occ;

                f(TOTAL_THREATS + [0, 384][side] + 64 * (piece - 2) + sq);
                map_bb(threats, |dest| {
                    let enemy = (1 << dest) & opps > 0;
                    if let Some(idx) = map_piece_threat(piece, sq, dest, pieces[dest], enemy) {
                        f(side_offset + idx);
                    }
                });
            });
        }
    }
}

fn map_bb<F: FnMut(usize)>(mut bb: u64, mut f: F) {
    while bb > 0 {
        let sq = bb.trailing_zeros() as usize;
        f(sq);
        bb &= bb - 1;
    }
}

fn flip_horizontal(mut bb: u64) -> u64 {
    const K1: u64 = 0x5555555555555555;
    const K2: u64 = 0x3333333333333333;
    const K4: u64 = 0x0f0f0f0f0f0f0f0f;
    bb = ((bb >> 1) & K1) | ((bb & K1) << 1);
    bb = ((bb >> 2) & K2) | ((bb & K2) << 2);
    ((bb >> 4) & K4) | ((bb & K4) << 4)
}

pub fn map_piece_threat(
    piece: usize,
    src: usize,
    dest: usize,
    target: usize,
    enemy: bool,
) -> Option<usize> {
    match piece {
        Piece::PAWN => map_pawn_threat(src, dest, target, enemy),
        Piece::KNIGHT => map_knight_threat(src, dest, target),
        Piece::BISHOP => map_bishop_threat(src, dest, target),
        Piece::ROOK => map_rook_threat(src, dest, target),
        Piece::QUEEN => map_queen_threat(src, dest, target),
        Piece::KING => map_king_threat(src, dest, target),
        _ => unreachable!(),
    }
}

fn below(src: usize, dest: usize, table: &[u64; 64]) -> usize {
    (table[src] & ((1 << dest) - 1)).count_ones() as usize
}

const fn offset_mapping<const N: usize>(a: [usize; N]) -> [usize; 12] {
    let mut res = [usize::MAX; 12];

    let mut i = 0;
    while i < N {
        res[a[i] - 2] = i;
        res[a[i] + 4] = i + N;
        i += 1;
    }

    res
}

fn target_is(target: usize, piece: usize) -> bool {
    target % 6 == piece - 2
}

fn map_pawn_threat(src: usize, dest: usize, target: usize, enemy: bool) -> Option<usize> {
    const MAP: [usize; 12] = offset_mapping([Piece::PAWN, Piece::KNIGHT, Piece::ROOK]);
    if MAP[target] == usize::MAX || (enemy && dest > src && target_is(target, Piece::PAWN)) {
        None
    } else {
        let up = usize::from(dest > src);
        let diff = dest.abs_diff(src);
        let id = if diff == [9, 7][up] { 0 } else { 1 };
        let attack = 2 * (src % 8) + id - 1;
        let threat =
            ValueOffsets::PAWN + MAP[target] * ValueIndices::PAWN + (src / 8 - 1) * 14 + attack;

        assert!(threat < ValueOffsets::KNIGHT, "{threat}");

        Some(threat)
    }
}

fn map_knight_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    if dest > src && target_is(target, Piece::KNIGHT) {
        None
    } else {
        let idx = ValueIndices::KNIGHT[src] + below(src, dest, &ValueAttacks::KNIGHT);
        let threat = ValueOffsets::KNIGHT + target * ValueIndices::KNIGHT[64] + idx;

        assert!(threat >= ValueOffsets::KNIGHT, "{threat}");
        assert!(threat < ValueOffsets::BISHOP, "{threat}");

        Some(threat)
    }
}

fn map_bishop_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    const MAP: [usize; 12] = offset_mapping([
        Piece::PAWN,
        Piece::KNIGHT,
        Piece::BISHOP,
        Piece::ROOK,
        Piece::KING,
    ]);
    if MAP[target] == usize::MAX || dest > src && target_is(target, Piece::BISHOP) {
        None
    } else {
        let idx = ValueIndices::BISHOP[src] + below(src, dest, &ValueAttacks::BISHOP);
        let threat = ValueOffsets::BISHOP + MAP[target] * ValueIndices::BISHOP[64] + idx;

        assert!(threat >= ValueOffsets::BISHOP, "{threat}");
        assert!(threat < ValueOffsets::ROOK, "{threat}");

        Some(threat)
    }
}

fn map_rook_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    const MAP: [usize; 12] = offset_mapping([
        Piece::PAWN,
        Piece::KNIGHT,
        Piece::BISHOP,
        Piece::ROOK,
        Piece::KING,
    ]);
    if MAP[target] == usize::MAX || dest > src && target_is(target, Piece::ROOK) {
        None
    } else {
        let idx = ValueIndices::ROOK[src] + below(src, dest, &ValueAttacks::ROOK);
        let threat = ValueOffsets::ROOK + MAP[target] * ValueIndices::ROOK[64] + idx;

        assert!(threat >= ValueOffsets::ROOK, "{threat}");
        assert!(threat < ValueOffsets::QUEEN, "{threat}");

        Some(threat)
    }
}

fn map_queen_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    if dest > src && target_is(target, Piece::QUEEN) {
        None
    } else {
        let idx = ValueIndices::QUEEN[src] + below(src, dest, &ValueAttacks::QUEEN);
        let threat = ValueOffsets::QUEEN + target * ValueIndices::QUEEN[64] + idx;

        assert!(threat >= ValueOffsets::QUEEN, "{threat}");
        assert!(threat < ValueOffsets::KING, "{threat}");

        Some(threat)
    }
}

fn map_king_threat(src: usize, dest: usize, target: usize) -> Option<usize> {
    const MAP: [usize; 12] =
        offset_mapping([Piece::PAWN, Piece::KNIGHT, Piece::BISHOP, Piece::ROOK]);
    if MAP[target] == usize::MAX {
        None
    } else {
        let idx = ValueIndices::KING[src] + below(src, dest, &ValueAttacks::KING);
        let threat = ValueOffsets::KING + MAP[target] * ValueIndices::KING[64] + idx;

        assert!(threat >= ValueOffsets::KING, "{threat}");
        assert!(threat < ValueOffsets::END, "{threat}");

        Some(threat)
    }
}
