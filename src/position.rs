use super::{
    attacks::Attacks,
    consts::{CASTLE_MASK, Flag, IN_BETWEEN, LINE_THROUGH, Path, Piece, ROOK_MOVES, Rank, Right, Side},
};

macro_rules! pop_lsb {
    ($idx:ident, $x:expr) => {
        let $idx = $x.trailing_zeros() as u8;
        $x &= $x - 1
    };
}

#[derive(Copy, Clone, Default)]
pub struct Position {
    bb: [u64; 8],
    stm: bool,
    enp_sq: u8,
    rights: u8,
}

#[derive(Copy, Clone, Default)]
pub struct Move {
    from: u8,
    to: u8,
    flag: u8,
    moved: u8,
}

pub struct MoveList {
    pub list: [Move; 252],
    pub len: usize,
}

impl Move {
    #[must_use]
    pub fn new(from: u8, to: u8, flag: u8, moved: u8) -> Self {
        Self {
            from,
            to,
            flag,
            moved,
        }
    }

    #[must_use]
    pub fn to_uci(self) -> String {
        let idx_to_sq = |i| format!("{}{}", ((i & 7) + b'a') as char, (i / 8) + 1);
        let promo = if self.flag & 0b1000 > 0 {
            ["n", "b", "r", "q"][(self.flag & 0b11) as usize]
        } else {
            ""
        };
        format!("{}{}{}", idx_to_sq(self.from), idx_to_sq(self.to), promo)
    }
}

impl std::ops::Deref for MoveList {
    type Target = [Move];
    fn deref(&self) -> &Self::Target {
        &self.list[..self.len]
    }
}

impl MoveList {
    #[inline]
    fn push(&mut self, from: u8, to: u8, flag: u8, mpc: usize) {
        self.list[self.len] = Move::new(from, to, flag, mpc as u8);
        self.len += 1;
    }
}

#[inline]
fn encode<const PC: usize, const FLAG: u8>(moves: &mut MoveList, mut attacks: u64, from: u8) {
    while attacks > 0 {
        pop_lsb!(to, attacks);

        moves.push(from, to, FLAG, PC);
    }
}

impl Position {
    // ACCESSOR METHODS

    #[must_use]
    pub fn piece(&self, piece: usize) -> u64 {
        self.bb[piece]
    }

    #[must_use]
    pub fn stm(&self) -> usize {
        usize::from(self.stm)
    }

    #[must_use]
    pub fn rights(&self) -> u8 {
        self.rights
    }

    #[must_use]
    pub fn enp_sq(&self) -> u8 {
        self.enp_sq
    }

    // POSITION INFO

    #[must_use]
    pub fn occ(&self) -> u64 {
        self.bb[Side::WHITE] | self.bb[Side::BLACK]
    }

    #[must_use]
    pub fn king_index(&self) -> usize {
        (self.bb[Piece::KING] & self.bb[usize::from(self.stm)]).trailing_zeros() as usize
    }

    #[must_use]
    pub fn boys(&self) -> u64 {
        self.bb[usize::from(self.stm)]
    }

    #[must_use]
    pub fn opps(&self) -> u64 {
        self.bb[usize::from(!self.stm)]
    }

    #[must_use]
    pub fn attackers_to_square(&self, sq: usize, side: usize, occ: u64) -> u64 {
        ((Attacks::knight(sq) & self.bb[Piece::KNIGHT])
            | (Attacks::king(sq) & self.bb[Piece::KING])
            | (Attacks::pawn(sq, side) & self.bb[Piece::PAWN])
            | (Attacks::rook(sq, occ) & (self.bb[Piece::ROOK] ^ self.bb[Piece::QUEEN]))
            | (Attacks::bishop(sq, occ) & (self.bb[Piece::BISHOP] ^ self.bb[Piece::QUEEN])))
            & self.bb[side ^ 1]
    }

    #[must_use]
    pub fn is_square_attacked(&self, sq: usize, side: usize, occ: u64) -> bool {
        self.attackers_to_square(sq, side, occ) > 0
    }

    #[must_use]
    pub fn get_pc(&self, bit: u64) -> usize {
        for pc in Piece::PAWN..=Piece::QUEEN {
            if bit & self.bb[pc] > 0 {
                return pc;
            }
        }
        0
    }

    // MODIFY POSITION

    pub fn toggle(&mut self, side: usize, piece: usize, bit: u64) {
        self.bb[piece] ^= bit;
        self.bb[side] ^= bit;
    }

    pub fn make(&mut self, mov: Move) {
        // extracting move info
        let side = usize::from(self.stm);
        let bb_from = 1 << mov.from;
        let bb_to = 1 << mov.to;
        let captured = if mov.flag & Flag::CAP == 0 {
            Piece::EMPTY
        } else {
            self.get_pc(bb_to)
        };

        // updating state
        self.stm = !self.stm;
        self.enp_sq = 0;
        self.rights &= CASTLE_MASK[usize::from(mov.to)] & CASTLE_MASK[usize::from(mov.from)];

        // move piece
        self.toggle(side, usize::from(mov.moved), bb_from ^ bb_to);

        // captures
        if captured != Piece::EMPTY {
            self.toggle(side ^ 1, captured, bb_to);
        }

        // more complex moves
        match mov.flag {
            Flag::DBL => self.enp_sq = mov.to ^ 8,
            Flag::KS | Flag::QS => {
                let bits = ROOK_MOVES[usize::from(mov.flag == Flag::KS)][side];
                self.toggle(side, Piece::ROOK, bits);
            }
            Flag::ENP => {
                let bits = 1 << (mov.to ^ 8);
                self.toggle(side ^ 1, Piece::PAWN, bits);
            }
            Flag::NPR.. => {
                let promo = usize::from((mov.flag & 3) + 3);
                self.bb[Piece::PAWN] ^= bb_to;
                self.bb[promo] ^= bb_to;
            }
            _ => {}
        }
    }

    // CREATE POSITION

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
                pos.toggle(colour, idx + 2 - 6 * colour, 1 << (8 * row + col));
                col += 1;
            }
        }

        // side to move
        pos.stm = vec[1] == "b";

        // castle rights
        for ch in vec[2].chars() {
            pos.rights |= match ch {
                'Q' => Right::WQS,
                'K' => Right::WKS,
                'q' => Right::BQS,
                'k' => Right::BKS,
                _ => 0,
            }
        }

        // en passant square
        pos.enp_sq = if vec[3] == "-" {
            0
        } else {
            let chs: Vec<char> = vec[3].chars().collect();
            8 * chs[1].to_string().parse::<u8>().unwrap_or(0) + chs[0] as u8 - 105
        };

        pos
    }

    #[must_use]
    pub fn gen(&self) -> MoveList {
        let mut moves = MoveList {
            list: [Move::default(); 252],
            len: 0,
        };

        let checkers = self.checkers();
        let pinned = self.pinned();
        let king_sq = self.king_index();

        self.king_moves(&mut moves);

        if checkers == 0 {
            self.gen_pnbrq(&mut moves, u64::MAX, u64::MAX, pinned);
            self.castles(&mut moves, self.occ());
        } else if checkers & (checkers - 1) == 0 {
            let checker_sq = checkers.trailing_zeros() as usize;
            let free = IN_BETWEEN[king_sq][checker_sq];
            self.gen_pnbrq(&mut moves, checkers, free, pinned);
        }

        moves
    }

    fn king_moves(&self, moves: &mut MoveList) {
        let king_sq = self.king_index();
        let attacks = Attacks::king(king_sq);
        let side = self.stm();
        let occ = self.occ();
        let no_king = occ ^ (1 << king_sq);

        let mut caps = attacks & self.opps();
        while caps > 0 {
            pop_lsb!(to, caps);

            if !self.is_square_attacked(usize::from(to), side, no_king) {
                moves.push(king_sq as u8, to, Flag::CAP, Piece::KING);
            }
        }

        let mut quiets = attacks & !occ;
        while quiets > 0 {
            pop_lsb!(to, quiets);

            if !self.is_square_attacked(usize::from(to), side, no_king) {
                moves.push(king_sq as u8, to, Flag::QUIET, Piece::KING);
            }
        }
    }

    fn gen_pnbrq(&self, moves: &mut MoveList, checkers: u64, free: u64, pinned: u64) {
        let boys = self.boys();
        let pawns = self.piece(Piece::PAWN) & boys;
        let side = self.stm();
        let pinned_pawns = pawns & pinned;
        let free_pawns = pawns & !pinned;
        let check_mask = free | checkers;

        if side == Side::WHITE {
            self.pawn_pushes::<{ Side::WHITE }, false>(moves, free_pawns, free);
            self.pawn_pushes::<{ Side::WHITE }, true>(moves, pinned_pawns, free);
        } else {
            self.pawn_pushes::<{ Side::BLACK }, false>(moves, free_pawns, free);
            self.pawn_pushes::<{ Side::BLACK }, true>(moves, pinned_pawns, free);
        }

        if self.enp_sq() > 0 {
            self.en_passants(moves, pawns);
        }

        self.pawn_captures::<false>(moves, free_pawns, checkers);
        self.pawn_captures::<true>(moves, pinned_pawns, checkers);

        self.piece_moves::<{ Piece::KNIGHT }>(moves, check_mask, pinned);
        self.piece_moves::<{ Piece::BISHOP }>(moves, check_mask, pinned);
        self.piece_moves::<{ Piece::ROOK }>(moves, check_mask, pinned);
        self.piece_moves::<{ Piece::QUEEN }>(moves, check_mask, pinned);
    }

    fn castles(&self, moves: &mut MoveList, occ: u64) {
        if self.stm() == Side::BLACK {
            if self.can_castle::<{ Side::BLACK }, 0>(occ, 59, 58) {
                moves.push(60, 58, Flag::QS, Piece::KING);
            }
            if self.can_castle::<{ Side::BLACK }, 1>(occ, 61, 62) {
                moves.push(60, 62, Flag::KS, Piece::KING);
            }
        } else {
            if self.can_castle::<{ Side::WHITE }, 0>(occ, 3, 2) {
                moves.push(4, 2, Flag::QS, Piece::KING);
            }
            if self.can_castle::<{ Side::WHITE }, 1>(occ, 5, 6) {
                moves.push(4, 6, Flag::KS, Piece::KING);
            }
        }
    }

    fn can_castle<const SIDE: usize, const KS: usize>(
        &self,
        occ: u64,
        sq1: usize,
        sq2: usize,
    ) -> bool {
        self.rights() & Right::TABLE[SIDE][KS] > 0
            && occ & Path::TABLE[SIDE][KS] == 0
            && !self.is_square_attacked(sq1, SIDE, occ)
            && !self.is_square_attacked(sq2, SIDE, occ)
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
        let rq = self.piece(Piece::QUEEN) | self.piece(Piece::ROOK);
        let bq = self.piece(Piece::QUEEN) | self.piece(Piece::BISHOP);

        let mut pinned = 0;

        let mut pinners = Attacks::xray_rook(kidx, occ, boys) & opps & rq;
        while pinners > 0 {
            pop_lsb!(sq, pinners);
            pinned |= IN_BETWEEN[usize::from(sq)][kidx] & boys;
        }

        pinners = Attacks::xray_bishop(kidx, occ, boys) & opps & bq;
        while pinners > 0 {
            pop_lsb!(sq, pinners);
            pinned |= IN_BETWEEN[usize::from(sq)][kidx] & boys;
        }

        pinned
    }

    fn piece_moves<const PC: usize>(&self, moves: &mut MoveList, check_mask: u64, pinned: u64) {
        let attackers = self.boys() & self.piece(PC);
        self.piece_moves_internal::<PC, false>(moves, check_mask, attackers & !pinned);
        self.piece_moves_internal::<PC, true>(moves, check_mask, attackers & pinned);
    }

    fn piece_moves_internal<const PC: usize, const PINNED: bool>(
        &self,
        moves: &mut MoveList,
        check_mask: u64,
        mut attackers: u64,
    ) {
        let occ = self.occ();
        let king_sq = self.king_index();

        while attackers > 0 {
            pop_lsb!(from, attackers);

            let mut attacks = match PC {
                Piece::KNIGHT => Attacks::knight(usize::from(from)),
                Piece::BISHOP => Attacks::bishop(usize::from(from), occ),
                Piece::ROOK => Attacks::rook(usize::from(from), occ),
                Piece::QUEEN => Attacks::queen(usize::from(from), occ),
                Piece::KING => Attacks::king(usize::from(from)),
                _ => unreachable!(),
            };

            attacks &= check_mask;

            if PINNED {
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            encode::<PC, { Flag::CAP }>(moves, attacks & self.opps(), from);
            encode::<PC, { Flag::QUIET }>(moves, attacks & !occ, from);
        }
    }

    fn pawn_captures<const PINNED: bool>(
        &self,
        moves: &mut MoveList,
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
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            encode::<{ Piece::PAWN }, { Flag::CAP }>(moves, attacks, from);
        }

        while promo_attackers > 0 {
            pop_lsb!(from, promo_attackers);

            let mut attacks = Attacks::pawn(usize::from(from), side) & opps & checkers;

            if PINNED {
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            while attacks > 0 {
                pop_lsb!(to, attacks);

                moves.push(from, to, Flag::QPC, Piece::PAWN);
                moves.push(from, to, Flag::NPC, Piece::PAWN);
                moves.push(from, to, Flag::BPC, Piece::PAWN);
                moves.push(from, to, Flag::RPC, Piece::PAWN);
            }
        }
    }

    fn pawn_pushes<const SIDE: usize, const PINNED: bool>(
        &self,
        moves: &mut MoveList,
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

            let to = idx_shift::<SIDE, 8>(from);

            if !PINNED || (1 << to) & LINE_THROUGH[king_sq][usize::from(from)] > 0 {
                moves.push(from, to, Flag::QUIET, Piece::PAWN);
            }
        }

        while promotable_pawns > 0 {
            pop_lsb!(from, promotable_pawns);

            let to = idx_shift::<SIDE, 8>(from);

            if !PINNED || (1 << to) & LINE_THROUGH[king_sq][usize::from(from)] > 0 {
                moves.push(from, to, Flag::QPR, Piece::PAWN);
                moves.push(from, to, Flag::NPR, Piece::PAWN);
                moves.push(from, to, Flag::BPR, Piece::PAWN);
                moves.push(from, to, Flag::RPR, Piece::PAWN);
            }
        }

        let mut dbl_pushable_pawns =
            shift::<SIDE>(shift::<SIDE>(empty & Rank::DBL[SIDE] & check_mask) & empty) & pawns;

        while dbl_pushable_pawns > 0 {
            pop_lsb!(from, dbl_pushable_pawns);

            let to = idx_shift::<SIDE, 16>(from);

            if !PINNED || (1 << to) & LINE_THROUGH[king_sq][usize::from(from)] > 0 {
                moves.push(from, to, Flag::DBL, Piece::PAWN);
            }
        }
    }

    fn en_passants(&self, moves: &mut MoveList, pawns: u64) {
        let mut attackers = Attacks::pawn(usize::from(self.enp_sq()), self.stm() ^ 1) & pawns;

        while attackers > 0 {
            pop_lsb!(from, attackers);

            let mut tmp = *self;
            let mov = Move::new(from, self.enp_sq(), Flag::ENP, Piece::PAWN as u8);
            tmp.make(mov);

            let king = (tmp.piece(Piece::KING) & tmp.opps()).trailing_zeros() as usize;
            if !tmp.is_square_attacked(king, self.stm(), tmp.occ()) {
                moves.list[moves.len] = mov;
                moves.len += 1;
            }
        }
    }
}

fn shift<const SIDE: usize>(bb: u64) -> u64 {
    if SIDE == Side::WHITE {
        bb >> 8
    } else {
        bb << 8
    }
}

fn idx_shift<const SIDE: usize, const AMOUNT: u8>(idx: u8) -> u8 {
    if SIDE == Side::WHITE {
        idx + AMOUNT
    } else {
        idx - AMOUNT
    }
}
