use crate::{game::GameState, pop_lsb};

use super::{
    attacks::Attacks,
    consts::*,
    moves::{serialise, Move},
};

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Board {
    bb: [u64; 8],
    hash: u64,
    stm: bool,
    halfm: u8,
}

impl Board {
    #[must_use]
    pub fn piece(&self, piece: usize) -> u64 {
        self.bb[piece]
    }

    pub fn bbs(&self) -> [u64; 8] {
        self.bb
    }

    #[must_use]
    pub fn stm(&self) -> usize {
        usize::from(self.stm)
    }

    #[must_use]
    pub fn halfm(&self) -> u8 {
        self.halfm
    }

    #[must_use]
    pub fn hash(&self) -> u64 {
        let mut hash = self.hash;

        if self.stm {
            hash ^= ZVALS.c;
        }

        hash
    }

    #[must_use]
    pub fn occ(&self) -> u64 {
        self.bb[Side::WHITE] | self.bb[Side::BLACK]
    }

    #[must_use]
    pub fn king_index(&self) -> usize {
        self.ksq(self.stm())
    }

    fn ksq(&self, side: usize) -> usize {
        (self.bb[Piece::KING] & self.bb[side]).trailing_zeros() as usize
    }

    #[must_use]
    pub fn boys(&self) -> u64 {
        self.bb[usize::from(self.stm)]
    }

    #[must_use]
    pub fn opps(&self) -> u64 {
        self.bb[usize::from(!self.stm)]
    }

    pub fn in_check(&self) -> bool {
        let king = (self.piece(Piece::KING) & self.boys()).trailing_zeros();
        self.is_square_attacked(king as usize, self.stm(), self.occ())
    }

    fn repetition(&self, stack: &[u64]) -> bool {
        let curr_hash = self.hash();

        for &hash in stack
            .iter()
            .rev()
            .take(self.halfm as usize + 1)
            .skip(1)
            .step_by(2)
        {
            if hash == curr_hash {
                return true;
            }
        }

        false
    }

    fn draw(&self) -> bool {
        self.halfm >= 100 || self.occ().count_ones() == 2
    }

    pub fn game_state(&self, stack: &[u64]) -> GameState {
        if self.draw() || self.repetition(stack) {
            return GameState::Draw;
        }

        if self.opps().count_ones() == 1 {
            return GameState::Won;
        }

        if self.boys().count_ones() == 1 {
            return if self.opps().count_ones() > 2 {
                GameState::Lost
            } else {
                let boy = self.ksq(self.stm());
                let opp = self.ksq(self.stm() ^ 1);
                let remaining = self.opps() & !self.bb[Piece::KING];

                if Attacks::king(boy) & remaining == 0 {
                    GameState::Lost
                } else if Attacks::king(opp) & remaining == 0 {
                    GameState::Draw
                } else {
                    GameState::Lost
                }
            };
        }

        let mut count = 0;
        self.map_legal_moves(&mut |_| count += 1);

        if count > 0 {
            return GameState::Ongoing;
        }

        if self.in_check() {
            GameState::Lost
        } else {
            GameState::Draw
        }
    }

    pub fn features_map<F: FnMut(usize)>(&self, mut f: F) {
        let boys_occ = self.boys();
        let opps_occ = self.opps();

        for (pc, bb) in self.bb.iter().skip(Piece::PAWN).enumerate() {
            assert!(pc < 6);

            let mut boys = bb & boys_occ;
            let mut opps = bb & opps_occ;

            if self.stm {
                boys = boys.swap_bytes();
                opps = opps.swap_bytes();
            }

            while boys > 0 {
                pop_lsb!(sq, boys);
                f(64 * pc + usize::from(sq));
            }

            while opps > 0 {
                pop_lsb!(sq, opps);
                f(384 + 64 * pc + usize::from(sq));
            }
        }
    }

    #[must_use]
    pub fn attackers_to_square(&self, sq: usize, side: usize, occ: u64) -> u64 {
        ((Attacks::knight(sq) & self.bb[Piece::KNIGHT])
            | (Attacks::king(sq) & self.bb[Piece::KING])
            | (Attacks::pawn(sq, side) & self.bb[Piece::PAWN])
            | (Attacks::rook(sq, occ) & self.bb[Piece::ROOK])
            | (Attacks::bishop(sq) & self.bb[Piece::BISHOP])
            | (Attacks::queen(sq) & self.bb[Piece::QUEEN]))
            & self.bb[side ^ 1]
    }

    #[must_use]
    pub fn is_square_attacked(&self, sq: usize, side: usize, occ: u64) -> bool {
        self.attackers_to_square(sq, side, occ) > 0
    }

    #[must_use]
    pub fn get_pc(&self, bit: u64) -> usize {
        for pc in Piece::PAWN..=Piece::KING {
            if bit & self.bb[pc] > 0 {
                return pc;
            }
        }

        0
    }

    pub fn flip_val(&self) -> u16 {
        [0, 56][self.stm()]
    }

    #[must_use]
    pub fn threats(&self) -> u64 {
        let mut threats = 0;

        let king = self.piece(Piece::KING) & self.boys();
        let occ = self.occ() ^ king;

        let side = self.stm() ^ 1;
        let opps = self.bb[side];

        let mut rooks = opps & self.bb[Piece::ROOK];
        while rooks > 0 {
            pop_lsb!(sq, rooks);
            threats |= Attacks::rook(sq as usize, occ);
        }

        let mut bishops = opps & self.bb[Piece::BISHOP];
        while bishops > 0 {
            pop_lsb!(sq, bishops);
            threats |= Attacks::bishop(sq as usize);
        }

        let mut knights = opps & self.bb[Piece::KNIGHT];
        while knights > 0 {
            pop_lsb!(sq, knights);
            threats |= Attacks::knight(sq as usize);
        }

        let mut kings = opps & self.bb[Piece::KING];
        while kings > 0 {
            pop_lsb!(sq, kings);
            threats |= Attacks::king(sq as usize);
        }

        let mut queens = opps & self.bb[Piece::QUEEN];
        while queens > 0 {
            pop_lsb!(sq, queens);
            threats |= Attacks::queen(sq as usize);
        }

        let pawns = opps & self.bb[Piece::PAWN];
        threats |= if side == Side::WHITE {
            Attacks::white_pawn_setwise(pawns)
        } else {
            Attacks::black_pawn_setwise(pawns)
        };

        threats
    }

    pub fn toggle(&mut self, side: usize, piece: usize, sq: u8) {
        let bit = 1 << sq;
        self.bb[piece] ^= bit;
        self.bb[side] ^= bit;
        self.hash ^= ZVALS.pcs[side][piece][usize::from(sq)];
    }

    pub fn make(&mut self, mov: Move) {
        // extracting move info
        let side = usize::from(self.stm);
        let moved = self.get_pc(1 << mov.from());

        // updating state
        self.stm = !self.stm;
        self.halfm += 1;

        if moved == Piece::PAWN {
            self.halfm = 0;
        }

        // remove piece from source square
        self.toggle(side, moved, mov.from());

        // captures
        if mov.is_capture() {
            let captured = self.get_pc(1 << mov.to());
            assert_ne!(captured, Piece::KING, "attempted to capture king");
            self.halfm = 0;
            self.toggle(side ^ 1, captured, mov.to());
        }

        // place piece on destination square
        if mov.is_promo() {
            assert_eq!(moved, Piece::PAWN, "attempted to promote non-pawn");
            self.toggle(side, Piece::QUEEN, mov.to())
        } else {
            self.toggle(side, moved, mov.to())
        }
    }

    #[must_use]
    pub fn parse_fen(fen: &str) -> Self {
        let mut pos = Self::default();
        let vec: Vec<&str> = fen.split_whitespace().collect();
        let p: Vec<char> = vec[0].chars().collect();

        // board
        let (mut row, mut col) = (7, 0);
        for ch in p {
            if ch == '/' {
                row -= 1;
                col = 0;
            } else if ('1'..='8').contains(&ch) {
                col += ch.to_string().parse::<i16>().unwrap_or(0);
            } else {
                let idx: usize = "PNBRQKpnbrqk"
                    .chars()
                    .position(|element| element == ch)
                    .unwrap_or(6);
                let colour = usize::from(idx > 5);
                let pc = idx + 2 - 6 * colour;
                pos.toggle(colour, pc, (8 * row + col) as u8);
                col += 1;
            }
        }

        // side to move
        pos.stm = vec[1] == "b";

        pos
    }

    pub fn map_legal_moves<F: FnMut(Move)>(&self, f: &mut F) {
        let pinned = self.pinned();
        let king_sq = self.king_index();
        let threats = self.threats();
        let checkers = if threats & (1 << king_sq) > 0 {
            self.checkers()
        } else {
            0
        };

        self.king_moves::<F>(f, threats);

        if checkers == 0 {
            self.gen_pnbrq::<F>(f, u64::MAX, u64::MAX, pinned);
        } else if checkers & (checkers - 1) == 0 {
            let checker_sq = checkers.trailing_zeros() as usize;
            let free = in_between(king_sq, checker_sq);
            self.gen_pnbrq::<F>(f, checkers, free, pinned);
        }
    }

    fn king_moves<F: FnMut(Move)>(&self, f: &mut F, threats: u64) {
        let king_sq = self.king_index();
        let attacks = Attacks::king(king_sq) & !threats;
        let occ = self.occ();
        serialise(f, attacks & self.opps(), king_sq as u8, Flag::CAP);
        serialise(f, attacks & !occ, king_sq as u8, Flag::QUIET);
    }

    fn gen_pnbrq<F: FnMut(Move)>(&self, f: &mut F, checkers: u64, free: u64, pinned: u64) {
        let boys = self.boys();
        let pawns = self.piece(Piece::PAWN) & boys;
        let side = self.stm();
        let pinned_pawns = pawns & pinned;
        let free_pawns = pawns & !pinned;
        let check_mask = free | checkers;

        if side == Side::WHITE {
            self.pawn_pushes::<{ Side::WHITE }, false, F>(f, free_pawns, free);
            self.pawn_pushes::<{ Side::WHITE }, true, F>(f, pinned_pawns, free);
        } else {
            self.pawn_pushes::<{ Side::BLACK }, false, F>(f, free_pawns, free);
            self.pawn_pushes::<{ Side::BLACK }, true, F>(f, pinned_pawns, free);
        }

        self.pawn_captures::<false, F>(f, free_pawns, checkers);
        self.pawn_captures::<true, F>(f, pinned_pawns, checkers);

        self.piece_moves::<{ Piece::KNIGHT }, F>(f, check_mask, pinned);
        self.piece_moves::<{ Piece::BISHOP }, F>(f, check_mask, pinned);
        self.piece_moves::<{ Piece::ROOK }, F>(f, check_mask, pinned);
        self.piece_moves::<{ Piece::QUEEN }, F>(f, check_mask, pinned);
    }

    #[must_use]
    fn checkers(&self) -> u64 {
        self.attackers_to_square(self.king_index(), self.stm(), self.occ())
    }

    #[must_use]
    fn pinned(&self) -> u64 {
        let occ = self.occ();
        let boys = self.boys();
        let kidx = self.king_index();
        let opps = self.opps();
        let rooks = self.piece(Piece::ROOK);

        let mut pinned = 0;

        let mut pinners = Attacks::xray_rook(kidx, occ, boys) & opps & rooks;
        while pinners > 0 {
            pop_lsb!(sq, pinners);
            pinned |= in_between(usize::from(sq), kidx) & boys;
        }

        pinned
    }

    fn piece_moves<const PC: usize, F: FnMut(Move)>(
        &self,
        f: &mut F,
        check_mask: u64,
        pinned: u64,
    ) {
        let attackers = self.boys() & self.piece(PC);
        self.piece_moves_internal::<PC, false, F>(f, check_mask, attackers & !pinned);
        self.piece_moves_internal::<PC, true, F>(f, check_mask, attackers & pinned);
    }

    fn piece_moves_internal<const PC: usize, const PINNED: bool, F: FnMut(Move)>(
        &self,
        f: &mut F,
        check_mask: u64,
        mut attackers: u64,
    ) {
        let occ = self.occ();
        let king_sq = self.king_index();

        while attackers > 0 {
            pop_lsb!(from, attackers);

            let mut attacks = Attacks::of_piece::<PC>(usize::from(from), occ);

            attacks &= check_mask;

            if PINNED {
                attacks &= line_through(king_sq, usize::from(from));
            }

            serialise(f, attacks & self.opps(), from as u8, Flag::CAP);
            serialise(f, attacks & !occ, from as u8, Flag::QUIET);
        }
    }

    fn pawn_captures<const PINNED: bool, F: FnMut(Move)>(
        &self,
        f: &mut F,
        mut attackers: u64,
        checkers: u64,
    ) {
        let side = self.stm();
        let opps = self.opps();
        let king_sq = self.king_index();
        let mut promo_attackers = attackers & Rank::PEN[side];
        attackers &= !Rank::PEN[side];

        while attackers > 0 {
            pop_lsb!(from, attackers);

            let mut attacks = Attacks::pawn(usize::from(from), side) & opps & checkers;

            if PINNED {
                attacks &= line_through(king_sq, usize::from(from));
            }

            serialise(f, attacks, from as u8, Flag::CAP);
        }

        while promo_attackers > 0 {
            pop_lsb!(from, promo_attackers);

            let mut attacks = Attacks::pawn(usize::from(from), side) & opps & checkers;

            if PINNED {
                attacks &= line_through(king_sq, usize::from(from));
            }

            serialise(f, attacks, from as u8, Flag::CAP | Flag::PROMO);
        }
    }

    fn pawn_pushes<const SIDE: usize, const PINNED: bool, F: FnMut(Move)>(
        &self,
        f: &mut F,
        pawns: u64,
        check_mask: u64,
    ) {
        let empty = !self.occ();
        let king_sq = self.king_index();

        let mut pushable_pawns = shift::<SIDE>(empty & check_mask) & pawns;
        let mut promotable_pawns = pushable_pawns & Rank::PEN[SIDE];
        pushable_pawns &= !Rank::PEN[SIDE];

        while pushable_pawns > 0 {
            pop_lsb!(from, pushable_pawns);

            let to = idx_shift::<SIDE>(from);

            if !PINNED || (1 << to) & line_through(king_sq, usize::from(from)) > 0 {
                f(Move::new(from as u8, to, Flag::QUIET));
            }
        }

        while promotable_pawns > 0 {
            pop_lsb!(from, promotable_pawns);

            let to = idx_shift::<SIDE>(from);

            if !PINNED || (1 << to) & line_through(king_sq, usize::from(from)) > 0 {
                f(Move::new(from as u8, to, Flag::PROMO));
            }
        }
    }

    pub fn as_fen(&self) -> String {
        const PIECES: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut clear = 0;

            for file in 0..8 {
                let sq = 8 * rank + file;
                let bit = 1 << sq;
                let pc = self.get_pc(bit);
                if pc != 0 {
                    if clear > 0 {
                        fen.push_str(&format!("{}", clear));
                    }
                    clear = 0;
                    fen.push(PIECES[pc - 2 + 6 * usize::from(self.piece(Side::BLACK) & bit > 0)]);
                } else {
                    clear += 1;
                }
            }

            if clear > 0 {
                fen.push_str(&format!("{}", clear));
            }

            if rank > 0 {
                fen.push('/');
            }
        }

        fen.push(' ');
        fen.push(['w', 'b'][self.stm()]);
        fen.push_str(" 0 1");

        fen
    }
}

fn shift<const SIDE: usize>(bb: u64) -> u64 {
    if SIDE == Side::WHITE {
        bb >> 8
    } else {
        bb << 8
    }
}

fn idx_shift<const SIDE: usize>(idx: u16) -> u16 {
    if SIDE == Side::WHITE {
        idx + 8
    } else {
        idx - 8
    }
}

fn line_through(sq1: usize, sq2: usize) -> u64 {
    let file = sq1 % 8;

    if file == sq2 % 8 {
        return File::A << file;
    }

    if sq1 / 8 == sq2 / 8 {
        return 0xFF << (sq1 - file);
    }

    0
}

fn in_between(sq1: usize, sq2: usize) -> u64 {
    let on_same_file = sq1 % 8 == sq2 % 8;
    let on_same_rank = sq1 / 8 == sq2 / 8;

    if !(on_same_file | on_same_rank) {
        return 0;
    }

    let bit1 = 1 << sq1;
    let bit2 = 1 << sq2;

    let min = bit1.min(bit2);
    let mut btwn = (bit1.max(bit2) - min) ^ min;

    if on_same_file {
        btwn &= File::A << (sq1 % 8);
    }

    btwn
}
