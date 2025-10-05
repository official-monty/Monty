use montyformat::chess::{
    consts::{Rank, IN_BETWEEN, LINE_THROUGH},
    Attacks, Flag, Move, Piece, Position, Side,
};

use crate::pop_lsb;

pub const SEE_VALS: [i32; 8] = [0, 0, 100, 450, 450, 650, 1250, 0];

// This has been validated to be nearly fully legal (777/23441654 fails on lichess puzzles, 99.997% legal)
// See https://github.com/Viren6/Monty/tree/fully-legal-see-5 and
// https://huggingface.co/datasets/Viren6/SEE
pub fn greater_or_equal_to(pos: &Position, mov: &Move, threshold: i32) -> bool {
    let from = mov.src() as usize;
    let to = mov.to() as usize;
    let side = pos.stm();

    let moved_pc = pos.get_pc(1 << from);
    let captured_pc = if mov.is_en_passant() {
        Piece::PAWN
    } else {
        pos.get_pc(1 << to)
    };

    let from_bb = 1u64 << from;
    let ksq = pos.king_sq(side);
    let pinned = pos.pinned();
    if (pinned & from_bb) != 0 && (LINE_THROUGH[ksq][to] & from_bb) == 0 {
        return false;
    }

    let mut score = SEE_VALS[captured_pc] - threshold;

    if mov.is_promo() {
        let promo_val = SEE_VALS[mov.promo_pc()];
        score += promo_val - SEE_VALS[Piece::PAWN];
        if score < 0 {
            return false;
        }
        score -= promo_val;
        if score >= 0 {
            return true;
        }
    } else {
        if score < 0 {
            return false;
        }
        score -= SEE_VALS[moved_pc];
        if score >= 0 {
            let to_bb = 1u64 << to;
            let cap_sq = if mov.is_en_passant() { to ^ 8 } else { to };
            let cap_bb = 1u64 << cap_sq;

            // build board after the capture to check for further attackers
            let mut occ_after = pos.occ();
            occ_after ^= from_bb;
            occ_after ^= cap_bb;
            occ_after |= to_bb;
            let occ_att = occ_after ^ to_bb;

            let mut pieces_after = pos.bbs();
            pieces_after[moved_pc] ^= from_bb;
            pieces_after[side] ^= from_bb;
            if captured_pc != Piece::EMPTY {
                pieces_after[captured_pc] ^= cap_bb;
                pieces_after[side ^ 1] ^= cap_bb;
            }
            pieces_after[moved_pc] |= to_bb;
            pieces_after[side] |= to_bb;

            let opp = side ^ 1;
            let queens = pieces_after[Piece::QUEEN];
            let rooks = pieces_after[Piece::ROOK] | queens;
            let bishops = pieces_after[Piece::BISHOP] | queens;
            let pawns_w = pieces_after[Piece::PAWN] & pieces_after[Side::WHITE];
            let pawns_b = pieces_after[Piece::PAWN] & pieces_after[Side::BLACK];
            let mut opp_attackers = (Attacks::king(to) & pieces_after[Piece::KING])
                | (Attacks::knight(to) & pieces_after[Piece::KNIGHT])
                | (Attacks::bishop(to, occ_att) & bishops)
                | (Attacks::rook(to, occ_att) & rooks)
                | (Attacks::pawn(to, Side::WHITE) & pawns_b)
                | (Attacks::pawn(to, Side::BLACK) & pawns_w);
            opp_attackers &= pieces_after[opp];

            if opp_attackers == 0 {
                return true;
            }

            let promo_attackers = pieces_after[Piece::PAWN] & pieces_after[opp] & Rank::PEN[opp];
            if (Attacks::pawn(to, side) & promo_attackers) == 0 {
                return true;
            }
            let promo_penalty = SEE_VALS[Piece::QUEEN] - SEE_VALS[Piece::PAWN];
            if score >= promo_penalty {
                return true;
            }
            // if a pawn can recapture and promote, fall through to full
            // static exchange evaluation without modifying the score so
            // that further recaptures (e.g. king capturing the promoted
            // queen) are considered.
        }
    }

    let mut occ = pos.occ();
    let to_bb = 1u64 << to;
    occ &= !from_bb;
    occ &= !to_bb;
    if mov.is_en_passant() {
        occ &= !(1u64 << (to ^ 8));
    }

    if mov.flag() == Flag::DBL {
        let ep_sq = to ^ 8;
        let opp = side ^ 1;
        let mut ep_attackers = Attacks::pawn(ep_sq, side) & pos.piece(Piece::PAWN) & pos.piece(opp);
        if ep_attackers != 0 {
            let mut occ_after = pos.occ();
            occ_after ^= from_bb | (1u64 << to);
            let mut pieces_after = pos.bbs();
            pieces_after[Piece::PAWN] ^= from_bb | (1u64 << to);
            pieces_after[side] ^= from_bb | (1u64 << to);
            let pinned_opp = recompute_pins(&pieces_after, occ_after, opp, pos.king_sq(opp));
            ep_attackers &= !pinned_opp | (LINE_THROUGH[pos.king_sq(opp)][ep_sq] & pinned_opp);
            if ep_attackers != 0 {
                let mut legal = false;
                let mut attackers = ep_attackers;
                while attackers != 0 {
                    pop_lsb!(src, attackers);
                    let from_bit = 1u64 << src;
                    let mut occ_cap = occ_after ^ from_bit ^ (1u64 << to);
                    occ_cap |= 1u64 << ep_sq;
                    let mut pieces_cap = pieces_after;
                    pieces_cap[Piece::PAWN] ^= from_bit | (1u64 << to) | (1u64 << ep_sq);
                    pieces_cap[opp] ^= from_bit;
                    pieces_cap[opp] |= 1u64 << ep_sq;
                    pieces_cap[side] &= !(1u64 << to);

                    let king_sq = pos.king_sq(opp);
                    let queens = pieces_cap[Piece::QUEEN];
                    let rooks = pieces_cap[Piece::ROOK] | queens;
                    let bishops = pieces_cap[Piece::BISHOP] | queens;
                    let mut checkers = (Attacks::king(king_sq) & pieces_cap[Piece::KING])
                        | (Attacks::knight(king_sq) & pieces_cap[Piece::KNIGHT])
                        | (Attacks::bishop(king_sq, occ_cap) & bishops)
                        | (Attacks::rook(king_sq, occ_cap) & rooks)
                        | (Attacks::pawn(king_sq, opp) & pieces_cap[Piece::PAWN]);
                    checkers &= pieces_cap[side];
                    if checkers == 0 {
                        legal = true;
                        break;
                    }
                }
                if legal {
                    return threshold <= -SEE_VALS[Piece::PAWN];
                }
            }
        }
    }

    let mut pieces = pos.bbs();
    pieces[moved_pc] &= !from_bb;
    pieces[side] &= !from_bb;

    if captured_pc != Piece::EMPTY {
        let cap_sq = if mov.is_en_passant() { to ^ 8 } else { to };
        let cap_bb = 1u64 << cap_sq;
        pieces[captured_pc] &= !cap_bb;
        pieces[side ^ 1] &= !cap_bb;
    }

    // after making the move on the board, see if the opponent is in check. If they
    // are, they might be restricted in how they can recapture: in a double check or
    // if the checking piece isn't the one on the target square, only king captures
    // can be considered. We compute this information here and apply it after
    // generating the attackers.
    let mut pieces_after = pieces;
    let occ_after = occ | to_bb;
    pieces_after[moved_pc] |= to_bb;
    pieces_after[side] |= to_bb;

    let opp = side ^ 1;
    let ksq_opp = pos.king_sq(opp);
    let queens_after = pieces_after[Piece::QUEEN];
    let rooks_after = pieces_after[Piece::ROOK] | queens_after;
    let bishops_after = pieces_after[Piece::BISHOP] | queens_after;

    let mut checkers = (Attacks::king(ksq_opp) & pieces_after[Piece::KING])
        | (Attacks::knight(ksq_opp) & pieces_after[Piece::KNIGHT])
        | (Attacks::bishop(ksq_opp, occ_after) & bishops_after)
        | (Attacks::rook(ksq_opp, occ_after) & rooks_after)
        | (Attacks::pawn(ksq_opp, opp) & pieces_after[Piece::PAWN]);
    checkers &= pieces_after[side];

    let opp_in_check = checkers != 0;
    let double_check = checkers & (checkers - 1) != 0;
    let checker_on_to = (checkers & to_bb) != 0;

    let mut stm = side ^ 1;
    let mut attackers = {
        let queens = pieces[Piece::QUEEN];
        let rooks = pieces[Piece::ROOK] | queens;
        let bishops = pieces[Piece::BISHOP] | queens;
        let knights = pieces[Piece::KNIGHT];
        let kings = pieces[Piece::KING];
        let pawns_w = pieces[Piece::PAWN] & pieces[Side::WHITE];
        let pawns_b = pieces[Piece::PAWN] & pieces[Side::BLACK];
        (Attacks::king(to) & kings)
            | (Attacks::knight(to) & knights)
            | (Attacks::bishop(to, occ) & bishops)
            | (Attacks::rook(to, occ) & rooks)
            | (Attacks::pawn(to, Side::WHITE) & pawns_b)
            | (Attacks::pawn(to, Side::BLACK) & pawns_w)
    };

    if opp_in_check && (double_check || !checker_on_to) {
        attackers &= Attacks::king(to);
    }

    #[inline]
    fn recompute_pins(pieces: &[u64; 8], occ: u64, side: usize, ksq: usize) -> u64 {
        let boys = pieces[side];
        let opps = pieces[side ^ 1];
        let rq = pieces[Piece::QUEEN] | pieces[Piece::ROOK];
        let bq = pieces[Piece::QUEEN] | pieces[Piece::BISHOP];
        let mut pinned = 0u64;

        let mut pinners = Attacks::xray_rook(ksq, occ, boys) & opps & rq;
        while pinners > 0 {
            pop_lsb!(sq, pinners);
            pinned |= IN_BETWEEN[usize::from(sq)][ksq] & boys;
        }

        pinners = Attacks::xray_bishop(ksq, occ, boys) & opps & bq;
        while pinners > 0 {
            pop_lsb!(sq, pinners);
            pinned |= IN_BETWEEN[usize::from(sq)][ksq] & boys;
        }

        pinned
    }

    let mut pinned_w = recompute_pins(&pieces, occ, Side::WHITE, pos.king_sq(Side::WHITE));
    let mut pinned_b = recompute_pins(&pieces, occ, Side::BLACK, pos.king_sq(Side::BLACK));

    fn remove_least(
        pieces: &mut [u64; 8],
        mask: u64,
        occ: &mut u64,
        opp_king: usize,
        opp_pinned: u64,
        to: usize,
    ) -> Option<(usize, u64)> {
        const ORDER: [usize; 6] = [
            Piece::PAWN,
            Piece::KNIGHT,
            Piece::BISHOP,
            Piece::ROOK,
            Piece::QUEEN,
            Piece::KING,
        ];

        let mut global_fallback: Option<(usize, u64)> = None;

        for &pc in &ORDER {
            let mut bb = pieces[pc] & mask;
            if bb == 0 {
                continue;
            }

            // prefer moves that do not release pins on the opponent and do not
            // uncover x-ray attacks on the destination square
            let mut fallback_no_xray = None;

            while bb != 0 {
                let bit = bb & bb.wrapping_neg();
                bb ^= bit;
                let sq = bit.trailing_zeros() as usize;

                let releases_pin = (LINE_THROUGH[opp_king][sq] & opp_pinned) != 0;

                // check if moving this piece reveals a new slider attack on `to`
                let occ_after = *occ ^ bit;
                let side = if pieces[Side::WHITE] & bit != 0 {
                    Side::WHITE
                } else {
                    Side::BLACK
                };
                let opp = side ^ 1;
                let bishops = (pieces[Piece::BISHOP] | pieces[Piece::QUEEN]) & pieces[opp];
                let rooks = (pieces[Piece::ROOK] | pieces[Piece::QUEEN]) & pieces[opp];
                let pawns = pieces[Piece::PAWN] & pieces[opp];
                let pawn_attack = (Attacks::pawn(to, side) & pawns) != 0;
                let existing_xray = pawn_attack
                    || (Attacks::bishop(to, *occ) & bishops) != 0
                    || (Attacks::rook(to, *occ) & rooks) != 0;
                let opens_xray = !existing_xray
                    && ((Attacks::bishop(to, occ_after) & bishops) != 0
                        || (Attacks::rook(to, occ_after) & rooks) != 0);

                // If the attacker is a pawn on the promotion rank, prefer it even if
                // it releases a pin or opens an x-ray. Such captures are often the
                // only legal reply and ignoring them can dramatically skew the SEE.
                let promo_pawn = pc == Piece::PAWN && (bit & Rank::PEN[side]) != 0;

                if promo_pawn || (!releases_pin && !opens_xray) {
                    // best option: neither releases a pin nor opens an x-ray, or
                    // we must consider the promotion capture regardless
                    pieces[pc] ^= bit;
                    if pieces[Side::WHITE] & bit != 0 {
                        pieces[Side::WHITE] ^= bit;
                    } else {
                        pieces[Side::BLACK] ^= bit;
                    }
                    *occ ^= bit;
                    return Some((pc, bit));
                }

                if (!opens_xray || promo_pawn) && fallback_no_xray.is_none() {
                    fallback_no_xray = Some(bit);
                }

                if global_fallback.is_none() {
                    global_fallback = Some((pc, bit));
                }
            }

            if let Some(bit) = fallback_no_xray {
                pieces[pc] ^= bit;
                if pieces[Side::WHITE] & bit != 0 {
                    pieces[Side::WHITE] ^= bit;
                } else {
                    pieces[Side::BLACK] ^= bit;
                }
                *occ ^= bit;
                return Some((pc, bit));
            }
        }

        if let Some((pc, bit)) = global_fallback {
            pieces[pc] ^= bit;
            if pieces[Side::WHITE] & bit != 0 {
                pieces[Side::WHITE] ^= bit;
            } else {
                pieces[Side::BLACK] ^= bit;
            }
            *occ ^= bit;
            return Some((pc, bit));
        }

        None
    }

    while attackers & pieces[stm] != 0 {
        let allowed = {
            let all_pinned = pinned_w | pinned_b;
            let white_allowed = pinned_w & LINE_THROUGH[pos.king_sq(Side::WHITE)][to];
            let black_allowed = pinned_b & LINE_THROUGH[pos.king_sq(Side::BLACK)][to];
            !all_pinned | white_allowed | black_allowed
        };

        let our_attackers = attackers & pieces[stm] & allowed;
        let opp_pinned = if stm == Side::WHITE {
            pinned_b
        } else {
            pinned_w
        };
        let opp_king_sq = pos.king_sq(stm ^ 1);
        let Some((mut attacker_pc, from_bit)) = remove_least(
            &mut pieces,
            our_attackers,
            &mut occ,
            opp_king_sq,
            opp_pinned,
            to,
        ) else {
            break;
        };

        // after hypothetically moving this attacker to `to`, check if it leaves the king in check
        {
            let mut pieces_after = pieces;
            let occ_after = occ | to_bb;
            pieces_after[attacker_pc] |= to_bb;
            pieces_after[stm] |= to_bb;
            let ksq = if attacker_pc == Piece::KING {
                to
            } else {
                pos.king_sq(stm)
            };

            let queens = pieces_after[Piece::QUEEN];
            let rooks = pieces_after[Piece::ROOK] | queens;
            let bishops = pieces_after[Piece::BISHOP] | queens;
            let pawns_w = pieces_after[Piece::PAWN] & pieces_after[Side::WHITE];
            let pawns_b = pieces_after[Piece::PAWN] & pieces_after[Side::BLACK];
            let pawn_attacks = if stm == Side::WHITE {
                Attacks::pawn(ksq, Side::WHITE) & pawns_b
            } else {
                Attacks::pawn(ksq, Side::BLACK) & pawns_w
            };

            let mut checkers = (Attacks::king(ksq) & pieces_after[Piece::KING])
                | (Attacks::knight(ksq) & pieces_after[Piece::KNIGHT])
                | (Attacks::bishop(ksq, occ_after) & bishops)
                | (Attacks::rook(ksq, occ_after) & rooks)
                | pawn_attacks;
            checkers &= pieces_after[stm ^ 1];
            if checkers != 0 {
                // revert removal and skip this attacker
                pieces[attacker_pc] |= from_bit;
                pieces[stm] |= from_bit;
                occ |= from_bit;
                attackers &= !from_bit;
                continue;
            }
        }

        let capture_val = SEE_VALS[attacker_pc];
        if attacker_pc == Piece::PAWN
            && ((stm == Side::WHITE && to >= 56) || (stm == Side::BLACK && to < 8))
        {
            attacker_pc = Piece::QUEEN;
        }

        let queens = pieces[Piece::QUEEN];
        let rooks = pieces[Piece::ROOK] | queens;
        let bishops = pieces[Piece::BISHOP] | queens;

        if attacker_pc == Piece::PAWN || attacker_pc == Piece::BISHOP || attacker_pc == Piece::QUEEN
        {
            attackers |= Attacks::bishop(to, occ) & bishops;
        }
        if attacker_pc == Piece::ROOK || attacker_pc == Piece::QUEEN {
            attackers |= Attacks::rook(to, occ) & rooks;
        }

        attackers &= occ;

        if attacker_pc == Piece::KING && (attackers & pieces[stm ^ 1]) != 0 {
            break;
        }

        score = -score - 1 - capture_val;
        stm ^= 1;

        pinned_w = recompute_pins(&pieces, occ, Side::WHITE, pos.king_sq(Side::WHITE));
        pinned_b = recompute_pins(&pieces, occ, Side::BLACK, pos.king_sq(Side::BLACK));

        let promo_attackers = attackers & pieces[stm] & pieces[Piece::PAWN] & Rank::PEN[stm];
        if score >= 0 && promo_attackers == 0 {
            break;
        }
    }

    stm != side
}
