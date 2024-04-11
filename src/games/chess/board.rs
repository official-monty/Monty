use crate::{games::GameState, pop_lsb};

use super::{
    attacks::Attacks,
    consts::*,
    frc::Castling,
    moves::{serialise, Move},
};

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Board {
    bb: [u64; 8],
    hash: u64,
    phase: i32,
    stm: bool,
    enp_sq: u8,
    rights: u8,
    halfm: u8,
}

impl Board {
    pub fn from_raw(
        bb: [u64; 8],
        stm: bool,
        enp_sq: u8,
        rights: u8,
        halfm: u8,
    ) -> Self {
        Self {
            bb,
            hash: 0,
            phase: 0,
            stm,
            enp_sq,
            rights,
            halfm,
        }
    }

    // ACCESSOR METHODS

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
    pub fn rights(&self) -> u8 {
        self.rights
    }

    #[must_use]
    pub fn enp_sq(&self) -> u8 {
        self.enp_sq
    }

    #[must_use]
    pub fn halfm(&self) -> u8 {
        self.halfm
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

    pub fn game_state(&self, castling: &Castling, stack: &[u64]) -> GameState {
        if self.draw() || self.repetition(stack) {
            return GameState::Draw;
        }

        let mut count = 0;
        self.map_legal_moves(castling, |_| count += 1);

        if count > 0 {
            return GameState::Ongoing;
        }

        if self.in_check() {
            GameState::Lost(0)
        } else {
            GameState::Draw
        }
    }

    pub fn map_value_features<F: FnMut(usize)>(&self, f: F) {
        self.map_features::<F, true>(f);
    }

    pub fn map_policy_features<F: FnMut(usize)>(&self, f: F) {
        self.map_features::<F, false>(f);
    }

    fn map_features<F: FnMut(usize), const HM: bool>(&self, mut f: F) {
        let flip = self.stm() == Side::BLACK;
        let hm = if HM && self.king_index() % 8 > 3 {7} else {0};

        for piece in Piece::PAWN..=Piece::KING {
            let pc = 64 * (piece - 2);

            let mut our_bb = self.piece(piece) & self.piece(self.stm());
            let mut opp_bb = self.piece(piece) & self.piece(self.stm() ^ 1);

            if flip {
                our_bb = our_bb.swap_bytes();
                opp_bb = opp_bb.swap_bytes();
            }

            while our_bb > 0 {
                pop_lsb!(sq, our_bb);
                f(pc + usize::from(sq ^ hm));
            }

            while opp_bb > 0 {
                pop_lsb!(sq, opp_bb);
                f(384 + pc + usize::from(sq ^ hm));
            }
        }
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

    pub fn flip_val(&self) -> u16 {
        if self.stm() == Side::BLACK {
            56
        } else {
            0
        }
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
            self.get_pc(1 << mov.from())
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

    pub fn toggle(&mut self, side: usize, piece: usize, sq: u16) {
        let bit = 1 << sq;
        self.bb[piece] ^= bit;
        self.bb[side] ^= bit;
        self.hash ^= ZVALS.pcs[side][piece][usize::from(sq)];
    }

    pub fn make(&mut self, mov: Move, castling: &Castling) {
        // extracting move info
        let side = usize::from(self.stm);
        let bb_to = 1 << mov.to();
        let moved = self.get_pc(1 << mov.from());
        let captured = if !mov.is_capture() {
            Piece::EMPTY
        } else {
            self.get_pc(bb_to)
        };

        // updating state
        self.stm = !self.stm;
        self.enp_sq = 0;
        self.rights &=
            castling.mask(usize::from(mov.to())) & castling.mask(usize::from(mov.from()));
        self.halfm += 1;

        if moved == Piece::PAWN || mov.is_capture() {
            self.halfm = 0;
        }

        // move piece
        self.toggle(side, moved, mov.from());
        self.toggle(side, moved, mov.to());

        // captures
        if captured != Piece::EMPTY {
            self.toggle(side ^ 1, captured, mov.to());
            self.phase -= PHASE_VALS[captured];
        }

        // more complex moves
        match mov.flag() {
            Flag::DBL => self.enp_sq = mov.to() as u8 ^ 8,
            Flag::KS | Flag::QS => {
                let ks = usize::from(mov.flag() == Flag::KS);
                let sf = 56 * side as u16;
                let rfr = sf + castling.rook_file(side, ks);
                let rto = sf + [3, 5][ks];
                self.toggle(side, Piece::ROOK, rfr);
                self.toggle(side, Piece::ROOK, rto);
            }
            Flag::ENP => self.toggle(side ^ 1, Piece::PAWN, mov.to() ^ 8),
            Flag::NPR.. => {
                let promo = usize::from((mov.flag() & 3) + 3);
                self.phase += PHASE_VALS[promo];
                self.toggle(side, Piece::PAWN, mov.to());
                self.toggle(side, promo, mov.to());
            }
            _ => {}
        }
    }

    // CREATE POSITION

    #[must_use]
    pub fn parse_fen(fen: &str, castling: &mut Castling) -> Self {
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
                pos.toggle(colour, pc, (8 * row + col) as u16);
                pos.phase += PHASE_VALS[pc];
                col += 1;
            }
        }

        // side to move
        pos.stm = vec[1] == "b";

        // castle rights
        pos.rights = castling.parse(&pos, vec[2]);

        // en passant square
        pos.enp_sq = if vec[3] == "-" {
            0
        } else {
            let chs: Vec<char> = vec[3].chars().collect();
            8 * chs[1].to_string().parse::<u8>().unwrap_or(0) + chs[0] as u8 - 105
        };

        pos
    }

    pub fn map_legal_moves<F: FnMut(Move)>(&self, castling: &Castling, mut f: F) {
        self.map_legal_moves_internal::<true, F>(castling, &mut f);
    }

    pub fn map_legal_captures<F: FnMut(Move)>(&self, castling: &Castling, mut f: F) {
        self.map_legal_moves_internal::<false, F>(castling, &mut f);
    }

    fn map_legal_moves_internal<const QUIETS: bool, F: FnMut(Move)>(
        &self,
        castling: &Castling,
        f: &mut F,
    ) {
        let pinned = self.pinned();
        let king_sq = self.king_index();
        let threats = self.threats();
        let checkers = if threats & (1 << king_sq) > 0 {
            self.checkers()
        } else {
            0
        };

        self.king_moves::<QUIETS, F>(f, threats);

        if checkers == 0 {
            self.gen_pnbrq::<QUIETS, F>(f, u64::MAX, u64::MAX, pinned, castling);
            if QUIETS {
                self.castles(f, self.occ(), threats, castling, pinned);
            }
        } else if checkers & (checkers - 1) == 0 {
            let checker_sq = checkers.trailing_zeros() as usize;
            let free = IN_BETWEEN[king_sq][checker_sq];
            self.gen_pnbrq::<QUIETS, F>(f, checkers, free, pinned, castling);
        }
    }

    fn king_moves<const QUIETS: bool, F: FnMut(Move)>(&self, f: &mut F, threats: u64) {
        let king_sq = self.king_index();
        let attacks = Attacks::king(king_sq) & !threats;
        let occ = self.occ();

        let mut caps = attacks & self.opps();
        while caps > 0 {
            pop_lsb!(to, caps);
            f(Move::new(king_sq as u16, to, Flag::CAP));
        }

        if QUIETS {
            let mut quiets = attacks & !occ;
            while quiets > 0 {
                pop_lsb!(to, quiets);
                f(Move::new(king_sq as u16, to, Flag::QUIET));
            }
        }
    }

    fn gen_pnbrq<const QUIETS: bool, F: FnMut(Move)>(
        &self,
        f: &mut F,
        checkers: u64,
        free: u64,
        pinned: u64,
        castling: &Castling,
    ) {
        let boys = self.boys();
        let pawns = self.piece(Piece::PAWN) & boys;
        let side = self.stm();
        let pinned_pawns = pawns & pinned;
        let free_pawns = pawns & !pinned;
        let check_mask = free | checkers;

        if QUIETS {
            if side == Side::WHITE {
                self.pawn_pushes::<{ Side::WHITE }, false, F>(f, free_pawns, free);
                self.pawn_pushes::<{ Side::WHITE }, true, F>(f, pinned_pawns, free);
            } else {
                self.pawn_pushes::<{ Side::BLACK }, false, F>(f, free_pawns, free);
                self.pawn_pushes::<{ Side::BLACK }, true, F>(f, pinned_pawns, free);
            }
        }

        if self.enp_sq() > 0 {
            self.en_passants(f, pawns, castling);
        }

        self.pawn_captures::<false, F>(f, free_pawns, checkers);
        self.pawn_captures::<true, F>(f, pinned_pawns, checkers);

        self.piece_moves::<QUIETS, { Piece::KNIGHT }, F>(f, check_mask, pinned);
        self.piece_moves::<QUIETS, { Piece::BISHOP }, F>(f, check_mask, pinned);
        self.piece_moves::<QUIETS, { Piece::ROOK }, F>(f, check_mask, pinned);
        self.piece_moves::<QUIETS, { Piece::QUEEN }, F>(f, check_mask, pinned);
    }

    fn castles<F: FnMut(Move)>(
        &self,
        f: &mut F,
        occ: u64,
        threats: u64,
        castling: &Castling,
        pinned: u64,
    ) {
        let kbb = self.bb[Piece::KING] & self.bb[self.stm()];
        let ksq = kbb.trailing_zeros() as u16;

        let can_castle = |right: u8, kto: u64, rto: u64| {
            let side = self.stm();
            let ks = usize::from([Right::BKS, Right::WKS].contains(&right));
            let bit = 1 << (56 * side + usize::from(castling.rook_file(side, ks)));

            self.rights & right > 0
                && bit & pinned == 0
                && (occ ^ bit) & (btwn(kbb, kto) ^ kto) == 0
                && (occ ^ kbb) & (btwn(bit, rto) ^ rto) == 0
                && (btwn(kbb, kto) | kto) & threats == 0
        };

        if self.stm() == Side::BLACK {
            if can_castle(Right::BQS, 1 << 58, 1 << 59) {
                f(Move::new(ksq, 58, Flag::QS));
            }
            if can_castle(Right::BKS, 1 << 62, 1 << 61) {
                f(Move::new(ksq, 62, Flag::KS));
            }
        } else {
            if can_castle(Right::WQS, 1 << 2, 1 << 3) {
                f(Move::new(ksq, 2, Flag::QS));
            }
            if can_castle(Right::WKS, 1 << 6, 1 << 5) {
                f(Move::new(ksq, 6, Flag::KS));
            }
        }
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

    fn piece_moves<const QUIETS: bool, const PC: usize, F: FnMut(Move)>(
        &self,
        f: &mut F,
        check_mask: u64,
        pinned: u64,
    ) {
        let attackers = self.boys() & self.piece(PC);
        self.piece_moves_internal::<QUIETS, PC, false, F>(f, check_mask, attackers & !pinned);
        self.piece_moves_internal::<QUIETS, PC, true, F>(f, check_mask, attackers & pinned);
    }

    fn piece_moves_internal<
        const QUIETS: bool,
        const PC: usize,
        const PINNED: bool,
        F: FnMut(Move),
    >(
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
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            serialise(f, attacks & self.opps(), from, Flag::CAP);
            if QUIETS {
                serialise(f, attacks & !occ, from, Flag::QUIET);
            }
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
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            serialise(f, attacks, from, Flag::CAP);
        }

        while promo_attackers > 0 {
            pop_lsb!(from, promo_attackers);

            let mut attacks = Attacks::pawn(usize::from(from), side) & opps & checkers;

            if PINNED {
                attacks &= LINE_THROUGH[king_sq][usize::from(from)];
            }

            while attacks > 0 {
                pop_lsb!(to, attacks);

                f(Move::new(from, to, Flag::QPC));
                f(Move::new(from, to, Flag::NPC));
                f(Move::new(from, to, Flag::BPC));
                f(Move::new(from, to, Flag::RPC));
            }
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

            let to = idx_shift::<SIDE, 8>(from);

            if !PINNED || (1 << to) & LINE_THROUGH[king_sq][usize::from(from)] > 0 {
                f(Move::new(from, to, Flag::QUIET));
            }
        }

        while promotable_pawns > 0 {
            pop_lsb!(from, promotable_pawns);

            let to = idx_shift::<SIDE, 8>(from);

            if !PINNED || (1 << to) & LINE_THROUGH[king_sq][usize::from(from)] > 0 {
                f(Move::new(from, to, Flag::QPR));
                f(Move::new(from, to, Flag::NPR));
                f(Move::new(from, to, Flag::BPR));
                f(Move::new(from, to, Flag::RPR));
            }
        }

        let mut dbl_pushable_pawns =
            shift::<SIDE>(shift::<SIDE>(empty & Rank::DBL[SIDE] & check_mask) & empty) & pawns;

        while dbl_pushable_pawns > 0 {
            pop_lsb!(from, dbl_pushable_pawns);

            let to = idx_shift::<SIDE, 16>(from);

            if !PINNED || (1 << to) & LINE_THROUGH[king_sq][usize::from(from)] > 0 {
                f(Move::new(from, to, Flag::DBL));
            }
        }
    }

    fn en_passants<F: FnMut(Move)>(&self, f: &mut F, pawns: u64, castling: &Castling) {
        let mut attackers = Attacks::pawn(usize::from(self.enp_sq()), self.stm() ^ 1) & pawns;

        while attackers > 0 {
            pop_lsb!(from, attackers);

            let mut tmp = *self;
            let mov = Move::new(from, u16::from(self.enp_sq()), Flag::ENP);
            tmp.make(mov, castling);

            let king = (tmp.piece(Piece::KING) & tmp.opps()).trailing_zeros() as usize;
            if !tmp.is_square_attacked(king, self.stm(), tmp.occ()) {
                f(mov);
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
        fen.push(' ');

        if self.rights == 0 {
            fen.push('-');
        } else {
            let mut r = self.rights;
            while r > 0 {
                let q = r.trailing_zeros();
                r &= r - 1;
                fen.push(['k', 'q', 'K', 'Q'][q as usize]);
            }
        }

        fen.push_str(" - 0 1");

        fen
    }

    pub fn coloured_board(&self, counts: &[i32; 64], weights: &[f32; 64]) -> String {
        let pcs = [
            ['p', 'n', 'b', 'r', 'q', 'k'],
            ['P', 'N', 'B', 'R', 'Q', 'K'],
        ];

        let mut string = "+-----------------+\n".to_string();

        for i in (0..8).rev() {
            string += "|";

            for j in 0..8 {
                let sq = 8 * i + j;
                let pc = self.get_pc(1 << sq);
                let ch = if pc != 0 {
                    let is_white = self.piece(Side::WHITE) & (1 << sq) > 0;
                    pcs[usize::from(is_white)][pc - 2]
                } else {
                    '.'
                };

                if counts[sq] > 0 {
                    let g = (255.0 * (2.0 * weights[sq]).min(1.0)) as u8;
                    let r = 255 - g;
                    string += format!(" \x1b[38;2;{r};{g};0m{ch}\x1b[0m").as_str();
                } else {
                    string += format!(" \x1b[34m{ch}\x1b[0m").as_str();
                }
            }

            string += " |\n";
        }

        string += "+-----------------+";

        string
    }
}

fn shift<const SIDE: usize>(bb: u64) -> u64 {
    if SIDE == Side::WHITE {
        bb >> 8
    } else {
        bb << 8
    }
}

fn idx_shift<const SIDE: usize, const AMOUNT: u16>(idx: u16) -> u16 {
    if SIDE == Side::WHITE {
        idx + AMOUNT
    } else {
        idx - AMOUNT
    }
}

fn btwn(bit1: u64, bit2: u64) -> u64 {
    let min = bit1.min(bit2);
    (bit1.max(bit2) - min) ^ min
}
