use crate::{
    mcts::MctsParams,
    networks::{Accumulator, PolicyNetwork, ValueNetwork, POLICY_L1},
};

pub use montyformat::chess::{Attacks, Castling, GameState, Move, Position};

#[derive(Clone, Copy, Debug)]
pub struct EvalWdl {
    pub win: f32,
    pub draw: f32,
    pub loss: f32,
}

impl EvalWdl {
    pub fn new(win: f32, draw: f32, loss: f32) -> Self {
        let mut win = win.clamp(0.0, 1.0);
        let mut draw = draw.clamp(0.0, 1.0);
        let mut loss = loss.clamp(0.0, 1.0);

        let sum = win + draw + loss;

        if sum <= 0.0 {
            return Self {
                win: 1.0 / 3.0,
                draw: 1.0 / 3.0,
                loss: 1.0 / 3.0,
            };
        }

        let inv = 1.0 / sum;
        win *= inv;
        draw *= inv;
        loss *= inv;

        Self { win, draw, loss }
    }

    pub fn score(&self) -> f32 {
        self.win + 0.5 * self.draw
    }

    pub fn from_draw_and_score(draw: f32, score: f32) -> Self {
        let draw = draw.clamp(0.0, 1.0);
        let min_score = draw * 0.5;
        let max_score = 1.0 - draw * 0.5;
        let score = score.clamp(min_score.min(max_score), max_score.max(min_score));

        let win = (score - draw * 0.5).max(0.0);
        let loss = (1.0 - draw - win).max(0.0);
        Self::new(win, draw, loss)
    }

    pub fn to_cp_i32(&self) -> i32 {
        const K: f32 = 400.0;
        let score = self.score().clamp(0.0, 1.0);
        (-K * (1.0 / score - 1.0).ln()) as i32
    }

    pub fn apply_contempt(self, contempt: f32) -> Self {
        if contempt == 0.0 {
            return self;
        }

        let w = self.win;
        let l = self.loss;
        const EPS: f32 = 1e-4;

        if w <= EPS || l <= EPS || w >= 1.0 - EPS || l >= 1.0 - EPS {
            return self;
        }

        let a = (1.0 / l - 1.0).ln();
        let b = (1.0 / w - 1.0).ln();
        let denom = a + b;

        if !denom.is_finite() || denom.abs() < 1e-6 {
            return self;
        }

        let s = (2.0 / denom).clamp(-3.0, 3.0);
        let mu = (a - b) / denom;

        // Correction factor: 16x
        let delta_mu =
            (s * s * contempt * std::f32::consts::LN_10 / (400.0 * 16.0)).clamp(-0.8, 0.8);
        let mu_new = mu + delta_mu;

        let logistic = |x: f32| 1.0 / (1.0 + (-x).exp());
        let w_new = logistic((-1.0 + mu_new) / s);
        let l_new = logistic((-1.0 - mu_new) / s);
        let mut d_new = (1.0 - w_new - l_new).max(0.0);

        if d_new > 1.0 {
            d_new = 1.0;
        }

        EvalWdl::new(w_new, d_new, l_new)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EvalBreakdown {
    pub raw: EvalWdl,
    pub material: EvalWdl,
    pub contempt: EvalWdl,
    pub cp: i32,
}

#[derive(Clone)]
pub struct ChessState {
    board: Position,
    castling: Castling,
    stack: Vec<u64>,
}

impl Default for ChessState {
    fn default() -> Self {
        Self::from_fen(Self::STARTPOS)
    }
}

impl ChessState {
    pub const STARTPOS: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    #[cfg(feature = "datagen")]
    pub const BENCH_DEPTH: usize = 4;

    #[cfg(not(feature = "datagen"))]
    pub const BENCH_DEPTH: usize = 6;

    pub fn board(&self) -> Position {
        self.board
    }

    pub fn castling(&self) -> Castling {
        self.castling
    }

    pub fn conv_mov_to_str(&self, mov: Move) -> String {
        mov.to_uci(&self.castling)
    }

    pub fn from_fen(fen: &str) -> Self {
        let mut castling = Castling::default();
        let board = Position::parse_fen(fen, &mut castling);

        Self {
            board,
            castling,
            stack: Vec::new(),
        }
    }

    pub fn map_legal_moves<F: FnMut(Move)>(&self, f: F) {
        self.board.map_legal_moves(&self.castling, f);
    }

    pub fn game_state(&self) -> GameState {
        self.board.game_state(&self.castling, &self.stack)
    }

    pub fn hash(&self) -> u64 {
        self.board.hash()
    }

    pub fn make_move(&mut self, mov: Move) {
        self.stack.push(self.board.hash());
        self.board.make(mov, &self.castling);

        if self.board.halfm() == 0 {
            self.stack.clear();
        }
    }

    pub fn stm(&self) -> usize {
        self.board.stm()
    }

    pub fn map_moves_with_policies<F: FnMut(Move, f32)>(&self, policy: &PolicyNetwork, mut f: F) {
        let hl = policy.hl(&self.board);

        self.map_legal_moves(|mov| {
            let policy = policy.get(&self.board, &mov, &hl);
            f(mov, policy);
        });
    }

    pub fn get_policy_hl(&self, policy: &PolicyNetwork) -> Accumulator<i16, { POLICY_L1 / 2 }> {
        policy.hl(&self.board)
    }

    pub fn get_policy(
        &self,
        mov: Move,
        hl: &Accumulator<i16, { POLICY_L1 / 2 }>,
        policy: &PolicyNetwork,
    ) -> f32 {
        policy.get(&self.board, &mov, hl)
    }

    #[cfg(not(feature = "datagen"))]
    fn piece_count(&self, piece: usize) -> i32 {
        self.board.piece(piece).count_ones() as i32
    }

    fn evaluate_material_wdl(
        &self,
        value: &ValueNetwork,
        params: &MctsParams,
    ) -> (EvalWdl, EvalWdl, i32) {
        let (win, draw, loss) = value.eval(&self.board);
        let raw = EvalWdl::new(win, draw, loss);
        let cp_base = raw.to_cp_i32();

        #[cfg(not(feature = "datagen"))]
        let cp = {
            use montyformat::chess::consts::Piece;

            let mut mat = self.piece_count(Piece::KNIGHT) * params.knight_value()
                + self.piece_count(Piece::BISHOP) * params.bishop_value()
                + self.piece_count(Piece::ROOK) * params.rook_value()
                + self.piece_count(Piece::QUEEN) * params.queen_value();

            mat = params.material_offset() + mat / params.material_div1();

            cp_base * mat / params.material_div2()
        };

        #[cfg(feature = "datagen")]
        let cp = {
            let _ = params;
            cp_base
        };

        let score = 1.0 / (1.0 + (-(cp as f32) / 400.0).exp());
        let material = EvalWdl::from_draw_and_score(raw.draw, score);

        (raw, material, cp)
    }

    pub fn eval_with_contempt(
        &self,
        value: &ValueNetwork,
        params: &MctsParams,
        root_stm: usize,
    ) -> EvalBreakdown {
        let (raw, material, cp) = self.evaluate_material_wdl(value, params);
        let contempt = params.contempt() as f32;
        let perspective = if self.stm() == root_stm { 1.0 } else { -1.0 };
        let contempt_scaled = material.apply_contempt(contempt * perspective);

        EvalBreakdown {
            raw,
            material,
            contempt: contempt_scaled,
            cp,
        }
    }

    pub fn get_value(&self, value: &ValueNetwork, params: &MctsParams) -> i32 {
        let (_, _, cp) = self.evaluate_material_wdl(value, params);
        cp
    }

    pub fn get_value_wdl(&self, value: &ValueNetwork, params: &MctsParams, root_stm: usize) -> f32 {
        self.eval_with_contempt(value, params, root_stm)
            .contempt
            .score()
    }

    pub fn perft(&self, depth: usize) -> u64 {
        perft::<true, true>(&self.board, depth as u8, &self.castling)
    }

    pub fn display(&self, policy: &PolicyNetwork) {
        let mut moves = Vec::new();
        let mut max = f32::NEG_INFINITY;
        self.map_moves_with_policies(policy, |mov, policy| {
            moves.push((mov, policy));

            if policy > max {
                max = policy;
            }
        });

        let mut total = 0.0;

        for (_, policy) in moves.iter_mut() {
            *policy = (*policy - max).exp();
            total += *policy;
        }

        for (_, policy) in moves.iter_mut() {
            *policy /= total;
        }

        let mut w = [0f32; 64];
        let mut count = [0; 64];

        for &(mov, policy) in moves.iter() {
            let fr = usize::from(mov.src());
            let to = usize::from(mov.to());

            w[fr] = w[fr].max(policy);
            w[to] = w[to].max(policy);

            count[fr] += 1;
            count[to] += 1;
        }

        let pcs = [
            ['p', 'n', 'b', 'r', 'q', 'k'],
            ['P', 'N', 'B', 'R', 'Q', 'K'],
        ];

        println!("+-----------------+");

        for i in (0..8).rev() {
            print!("|");

            for j in 0..8 {
                let sq = 8 * i + j;
                let pc = self.board.get_pc(1 << sq);
                let ch = if pc != 0 {
                    let is_white = self.board.piece(0) & (1 << sq) > 0;
                    pcs[usize::from(is_white)][pc - 2]
                } else {
                    '.'
                };

                if count[sq] > 0 {
                    let g = (255.0 * (2.0 * w[sq]).min(1.0)) as u8;
                    let r = 255 - g;
                    print!(" \x1b[38;2;{r};{g};0m{ch}\x1b[0m");
                } else {
                    print!(" \x1b[34m{ch}\x1b[0m");
                }
            }

            println!(" |");
        }

        println!("+-----------------+");
    }
}

fn perft<const ROOT: bool, const BULK: bool>(
    pos: &Position,
    depth: u8,
    castling: &Castling,
) -> u64 {
    let mut count = 0;

    if BULK && !ROOT && depth == 1 {
        pos.map_legal_moves(castling, |_| count += 1);
    } else {
        let leaf = depth == 1;

        pos.map_legal_moves(castling, |mov| {
            let mut tmp = *pos;
            tmp.make(mov, castling);

            let num = if !BULK && leaf {
                1
            } else {
                perft::<false, BULK>(&tmp, depth - 1, castling)
            };

            count += num;

            if ROOT {
                println!("{}: {num}", mov.to_uci(castling));
            }
        });
    }

    count
}
