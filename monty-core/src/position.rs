use crate::{
    pop_lsb,
    value::{Accumulator, ValueNetwork},
    attacks::Attacks,
    consts::*,
    moves::{Move, MoveList},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameState {
    #[default]
    Ongoing,
    Lost,
    Draw,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Position {
    bb: [u64; 8],
    hash: u64,
    phase: i32,
    stm: bool,
    enp_sq: u8,
    rights: u8,
    halfm: u8,
}

pub struct FeatureList {
    list: [usize; 33],
    len: usize
}

impl Default for FeatureList {
    fn default() -> Self {
        Self { list: [0; 33], len: 0 }
    }
}

impl std::ops::Deref for FeatureList {
    type Target = [usize];
    fn deref(&self) -> &Self::Target {
        &self.list[..self.len]
    }
}

impl FeatureList {
    fn push(&mut self, feat: usize) {
        self.list[self.len] = feat;
        self.len += 1;
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

    #[must_use]
    pub fn hash(&self) -> u64 {
        let mut hash = self.hash;

        if self.enp_sq > 0 {
            hash ^= ZVALS.enp[self.enp_sq as usize & 7];
        }

        hash ^ ZVALS.cr[usize::from(self.rights)] ^ ZVALS.c[self.stm()]
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

    pub fn in_check(&self) -> bool {
        let king = (self.piece(Piece::KING) & self.boys()).trailing_zeros();
        self.is_square_attacked(king as usize, self.stm(), self.occ())
    }

    pub fn draw(&self) -> bool {
        if self.halfm >= 100 {
            return true;
        }

        let ph = self.phase;
        let b = self.bb[Piece::BISHOP];
        ph <= 2
            && self.bb[Piece::PAWN] == 0
            && ((ph != 2)
                || (b & self.bb[Side::WHITE] != b
                    && b & self.bb[Side::BLACK] != b
                    && (b & 0x55AA55AA55AA55AA == b || b & 0xAA55AA55AA55AA55 == b)))
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

    pub fn game_state(&self, moves: &MoveList, stack: &[u64]) -> GameState {
        if self.draw() || self.repetition(stack) {
            return GameState::Draw;
        }

        if moves.is_empty() {
            if self.in_check() {
                GameState::Lost
            } else {
                GameState::Draw
            }
        } else {
            GameState::Ongoing
        }
    }

    pub fn get_accs(&self) -> [Accumulator; 2] {
        let mut accs = [Accumulator::default(); 2];

        for side in [Side::WHITE, Side::BLACK] {
            for piece in Piece::PAWN..=Piece::KING {
                let mut bb = self.piece(piece) & self.bb[side];
                let pc = 64 * (piece - 2);

                while bb > 0 {
                    pop_lsb!(sq, bb);
                    accs[0].add_feature(384 * side + pc + sq as usize);
                    accs[1].add_feature(384 * (side ^ 1) + pc + (sq as usize ^ 56));
                }
            }
        }

        accs
    }

    pub fn get_features(&self) -> FeatureList {
        let flip = self.flip_val();
        let mut feats = FeatureList::default();
        feats.push(768);

        for piece in Piece::PAWN..=Piece::KING {
            let pc = 64 * (piece - 2);

            let mut our_bb = self.piece(piece) & self.piece(self.stm());
            while our_bb > 0 {
                pop_lsb!(sq, our_bb);
                feats.push(pc + usize::from(sq ^ flip));
            }

            let mut opp_bb = self.piece(piece) & self.piece(self.stm() ^ 1);
            while opp_bb > 0 {
                pop_lsb!(sq, opp_bb);
                feats.push(384 + pc + usize::from(sq ^ flip));
            }
        }

        feats
    }

    pub fn eval_from_acc(&self, accs: &[Accumulator; 2]) -> i32 {
        ValueNetwork::out(&accs[self.stm()], &accs[self.stm() ^ 1], self.occ())
    }

    pub fn eval_cp(&self) -> i32 {
        let accs = self.get_accs();
        self.eval_from_acc(&accs)
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
        for pc in Piece::PAWN..=Piece::KING {
            if bit & self.bb[pc] > 0 {
                return pc;
            }
        }
        0
    }

    pub fn flip_val(&self) -> u8 {
        if self.stm() == Side::BLACK { 56 } else { 0 }
    }

    #[must_use]
    pub fn threats(&self) -> u64 {
        let mut threats = 0;

        let king = self.piece(Piece::KING) & self.boys();
        let occ = self.occ() ^ king;

        let side = self.stm() ^ 1;
        let opps = self.bb[side];

        let queens = self.bb[Piece::QUEEN];

        let mut rooks = opps & (self.bb[Piece::ROOK] | queens);
        while rooks > 0 {
            pop_lsb!(sq, rooks);
            threats |= Attacks::rook(sq as usize, occ);
        }

        let mut bishops = opps & (self.bb[Piece::BISHOP] | queens);
        while bishops > 0 {
            pop_lsb!(sq, bishops);
            threats |= Attacks::bishop(sq as usize, occ);
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

        let pawns = opps & self.bb[Piece::PAWN];
        threats |= if side == Side::WHITE {
            Attacks::white_pawn_setwise(pawns)
        } else {
            Attacks::black_pawn_setwise(pawns)
        };

        threats
    }

    fn gain(&self, mov: &Move) -> i32 {
        if mov.is_en_passant() {
            return SEE_VALS[Piece::PAWN];
        }
        let mut score = SEE_VALS[self.get_pc(1 << mov.to())];
        if mov.is_promo() {
            score += SEE_VALS[mov.promo_pc()] - SEE_VALS[Piece::PAWN];
        }
        score
    }

    pub fn see(&self, mov: &Move, threshold: i32) -> bool {
        let sq = usize::from(mov.to());
        assert!(sq < 64, "wha");
        let mut next = if mov.is_promo() {
            mov.promo_pc()
        } else {
            usize::from(mov.moved())
        };
        let mut score = self.gain(mov) - threshold - SEE_VALS[next];

        if score >= 0 {
            return true;
        }

        let mut occ = (self.bb[Side::WHITE] | self.bb[Side::BLACK]) ^ (1 << mov.from()) ^ (1 << sq);
        if mov.is_en_passant() {
            occ ^= 1 << (sq ^ 8);
        }

        let bishops = self.bb[Piece::BISHOP] | self.bb[Piece::QUEEN];
        let rooks = self.bb[Piece::ROOK] | self.bb[Piece::QUEEN];
        let mut us = usize::from(!self.stm);
        let mut attackers = (Attacks::knight(sq) & self.bb[Piece::KNIGHT])
            | (Attacks::king(sq) & self.bb[Piece::KING])
            | (Attacks::pawn(sq, Side::WHITE) & self.bb[Piece::PAWN] & self.bb[Side::BLACK])
            | (Attacks::pawn(sq, Side::BLACK) & self.bb[Piece::PAWN] & self.bb[Side::WHITE])
            | (Attacks::rook(sq, occ) & rooks)
            | (Attacks::bishop(sq, occ) & bishops);

        loop {
            let our_attackers = attackers & self.bb[us];
            if our_attackers == 0 {
                break;
            }

            for pc in Piece::PAWN..=Piece::KING {
                let board = our_attackers & self.bb[pc];
                if board > 0 {
                    occ ^= board & board.wrapping_neg();
                    next = pc;
                    break;
                }
            }

            if [Piece::PAWN, Piece::BISHOP, Piece::QUEEN].contains(&next) {
                attackers |= Attacks::bishop(sq, occ) & bishops;
            }
            if [Piece::ROOK, Piece::QUEEN].contains(&next) {
                attackers |= Attacks::rook(sq, occ) & rooks;
            }

            attackers &= occ;
            score = -score - 1 - SEE_VALS[next];
            us ^= 1;

            if score >= 0 {
                if next == Piece::KING && attackers & self.bb[us] > 0 {
                    us ^= 1;
                }
                break;
            }
        }

        self.stm != (us == 1)
    }

    // MODIFY POSITION

    pub fn toggle<const ADD: bool>(&mut self, accs: &mut Option<&mut [Accumulator; 2]>, side: usize, piece: usize, sq: u8) {
        let bit = 1 << sq;
        self.bb[piece] ^= bit;
        self.bb[side] ^= bit;
        self.hash ^= ZVALS.pcs[side][piece][usize::from(sq)];

        if let Some(acc) = accs.as_mut() {
            let pc = 64 * (piece - 2);

            let start = 384 * side + pc + sq as usize;
            acc[0].update::<ADD>(start);

            let start = 384 * (side ^ 1) + pc + (sq ^ 56) as usize;
            acc[1].update::<ADD>(start);
        }
    }

    pub fn make(&mut self, mov: Move, mut acc: Option<&mut [Accumulator; 2]>) {
        // extracting move info
        let side = usize::from(self.stm);
        let bb_to = 1 << mov.to();
        let captured = if !mov.is_capture() {
            Piece::EMPTY
        } else {
            self.get_pc(bb_to)
        };

        // updating state
        self.stm = !self.stm;
        self.enp_sq = 0;
        self.rights &= CASTLE_MASK[usize::from(mov.to())] & CASTLE_MASK[usize::from(mov.from())];
        self.halfm += 1;

        if mov.moved() == Piece::PAWN as u8 || mov.is_capture() {
            self.halfm = 0;
        }

        // move piece
        self.toggle::<false>(&mut acc, side, usize::from(mov.moved()), mov.from());
        self.toggle::<true>(&mut acc, side, usize::from(mov.moved()), mov.to());

        // captures
        if captured != Piece::EMPTY {
            self.toggle::<false>(&mut acc, side ^ 1, captured, mov.to());
            self.phase -= PHASE_VALS[captured];
        }

        // more complex moves
        match mov.flag() {
            Flag::DBL => self.enp_sq = mov.to() ^ 8,
            Flag::KS | Flag::QS => {
                let (rfr, rto) = ROOK_MOVES[usize::from(mov.flag() == Flag::KS)][side];
                self.toggle::<false>(&mut acc, side, Piece::ROOK, rfr);
                self.toggle::<true>(&mut acc, side, Piece::ROOK, rto);
            }
            Flag::ENP => self.toggle::<false>(&mut acc, side ^ 1, Piece::PAWN, mov.to() ^ 8),
            Flag::NPR.. => {
                let promo = usize::from((mov.flag() & 3) + 3);
                self.phase += PHASE_VALS[promo];
                self.toggle::<false>(&mut acc, side, Piece::PAWN, mov.to());
                self.toggle::<true>(&mut acc, side, promo, mov.to());
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
                let pc = idx + 2 - 6 * colour;
                pos.toggle::<true>(&mut None, colour, pc, (8 * row + col) as u8);
                pos.phase += PHASE_VALS[pc];
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
    pub fn gen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::default();

        let pinned = self.pinned();
        let king_sq = self.king_index();
        let threats = self.threats();
        let checkers = if threats & (1 << king_sq) > 0 {
            self.checkers()
        } else {
            0
        };

        self.king_moves::<QUIETS>(&mut moves, threats);

        if checkers == 0 {
            self.gen_pnbrq::<QUIETS>(&mut moves, u64::MAX, u64::MAX, pinned);
            if QUIETS {
                self.castles(&mut moves, self.occ(), threats);
            }
        } else if checkers & (checkers - 1) == 0 {
            let checker_sq = checkers.trailing_zeros() as usize;
            let free = IN_BETWEEN[king_sq][checker_sq];
            self.gen_pnbrq::<QUIETS>(&mut moves, checkers, free, pinned);
        }

        moves
    }

    fn king_moves<const QUIETS: bool>(&self, moves: &mut MoveList, threats: u64) {
        let king_sq = self.king_index();
        let attacks = Attacks::king(king_sq) & !threats;
        let occ = self.occ();

        let mut caps = attacks & self.opps();
        while caps > 0 {
            pop_lsb!(to, caps);
            moves.push(king_sq as u8, to, Flag::CAP, Piece::KING);
        }

        if QUIETS {
            let mut quiets = attacks & !occ;
            while quiets > 0 {
                pop_lsb!(to, quiets);
                moves.push(king_sq as u8, to, Flag::QUIET, Piece::KING);
            }
        }
    }

    fn gen_pnbrq<const QUIETS: bool>(&self, moves: &mut MoveList, checkers: u64, free: u64, pinned: u64) {
        let boys = self.boys();
        let pawns = self.piece(Piece::PAWN) & boys;
        let side = self.stm();
        let pinned_pawns = pawns & pinned;
        let free_pawns = pawns & !pinned;
        let check_mask = free | checkers;

        if QUIETS {
            if side == Side::WHITE {
                self.pawn_pushes::<{ Side::WHITE }, false>(moves, free_pawns, free);
                self.pawn_pushes::<{ Side::WHITE }, true>(moves, pinned_pawns, free);
            } else {
                self.pawn_pushes::<{ Side::BLACK }, false>(moves, free_pawns, free);
                self.pawn_pushes::<{ Side::BLACK }, true>(moves, pinned_pawns, free);
            }
        }

        if self.enp_sq() > 0 {
            self.en_passants(moves, pawns);
        }

        self.pawn_captures::<false>(moves, free_pawns, checkers);
        self.pawn_captures::<true>(moves, pinned_pawns, checkers);

        self.piece_moves::<QUIETS, { Piece::KNIGHT }>(moves, check_mask, pinned);
        self.piece_moves::<QUIETS, { Piece::BISHOP }>(moves, check_mask, pinned);
        self.piece_moves::<QUIETS, { Piece::ROOK }>(moves, check_mask, pinned);
        self.piece_moves::<QUIETS, { Piece::QUEEN }>(moves, check_mask, pinned);
    }

    fn castles(&self, moves: &mut MoveList, occ: u64, threats: u64) {
        if self.stm() == Side::BLACK {
            if self.can_castle::<{ Side::BLACK }, 0>(occ, threats, 59, 58) {
                moves.push(60, 58, Flag::QS, Piece::KING);
            }
            if self.can_castle::<{ Side::BLACK }, 1>(occ, threats, 61, 62) {
                moves.push(60, 62, Flag::KS, Piece::KING);
            }
        } else {
            if self.can_castle::<{ Side::WHITE }, 0>(occ, threats, 3, 2) {
                moves.push(4, 2, Flag::QS, Piece::KING);
            }
            if self.can_castle::<{ Side::WHITE }, 1>(occ, threats, 5, 6) {
                moves.push(4, 6, Flag::KS, Piece::KING);
            }
        }
    }

    fn can_castle<const SIDE: usize, const KS: usize>(
        &self,
        occ: u64,
        threats: u64,
        sq1: usize,
        sq2: usize,
    ) -> bool {
        let path = (1 << sq1) | (1 << sq2);
        self.rights() & Right::TABLE[SIDE][KS] > 0
            && occ & Path::TABLE[SIDE][KS] == 0
            && path & threats == 0
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

    fn piece_moves<const QUIETS: bool, const PC: usize>(&self, moves: &mut MoveList, check_mask: u64, pinned: u64) {
        let attackers = self.boys() & self.piece(PC);
        self.piece_moves_internal::<QUIETS, PC, false>(moves, check_mask, attackers & !pinned);
        self.piece_moves_internal::<QUIETS, PC, true>(moves, check_mask, attackers & pinned);
    }

    fn piece_moves_internal<const QUIETS: bool, const PC: usize, const PINNED: bool>(
        &self,
        moves: &mut MoveList,
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
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            moves.serialise(attacks & self.opps(), from, Flag::CAP, PC);
            if QUIETS {
                moves.serialise(attacks & !occ, from, Flag::QUIET, PC);
            }
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

            moves.serialise(attacks, from, Flag::CAP, Piece::PAWN);
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
            tmp.make(mov, None);

            let king = (tmp.piece(Piece::KING) & tmp.opps()).trailing_zeros() as usize;
            if !tmp.is_square_attacked(king, self.stm(), tmp.occ()) {
                moves.push_raw(mov);
            }
        }
    }

    pub fn to_fen(&self) -> String {
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
        fen.push_str(" - - 0 1");

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

fn idx_shift<const SIDE: usize, const AMOUNT: u8>(idx: u8) -> u8 {
    if SIDE == Side::WHITE {
        idx + AMOUNT
    } else {
        idx - AMOUNT
    }
}

#[must_use]
pub fn perft<const ROOT: bool, const BULK: bool>(pos: &Position, depth: u8) -> u64 {
    let moves = pos.gen::<true>();

    if BULK && !ROOT && depth == 1 {
        return moves.len() as u64;
    }

    let mut positions = 0;
    let leaf = depth == 1;

    for m_idx in 0..moves.len() {
        let mut tmp = *pos;
        tmp.make(moves[m_idx], None);

        let num = if !BULK && leaf {
            1
        } else {
            perft::<false, BULK>(&tmp, depth - 1)
        };
        positions += num;

        if ROOT {
            println!("{}: {num}", moves[m_idx].to_uci());
        }
    }

    positions
}
