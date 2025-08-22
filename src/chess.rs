mod attacks;
mod board;
pub mod consts;
mod frc;
mod moves;

use crate::{
    mcts::MctsParams,
    networks::{Accumulator, PolicyNetwork, ValueNetwork, POLICY_L1},
};

pub use self::{attacks::Attacks, board::Board, frc::Castling, moves::Move};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameState {
    #[default]
    Ongoing,
    Lost(u8),
    Draw,
    Won(u8),
}

impl From<GameState> for u16 {
    fn from(value: GameState) -> Self {
        match value {
            GameState::Ongoing => 0,
            GameState::Draw => 1 << 8,
            GameState::Lost(x) => (2 << 8) ^ u16::from(x),
            GameState::Won(x) => (3 << 8) ^ u16::from(x),
        }
    }
}

impl From<u16> for GameState {
    fn from(value: u16) -> Self {
        let discr = value >> 8;
        let x = value as u8;

        match discr {
            0 => GameState::Ongoing,
            1 => GameState::Draw,
            2 => GameState::Lost(x),
            3 => GameState::Won(x),
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameState::Ongoing => write!(f, "O"),
            GameState::Lost(n) => write!(f, "L{n}"),
            GameState::Won(n) => write!(f, "W{n}"),
            GameState::Draw => write!(f, "D"),
        }
    }
}

#[derive(Clone)]
pub struct ChessState {
    board: Board,
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

    pub fn board(&self) -> Board {
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
        let board = Board::parse_fen(fen, &mut castling);

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

    pub fn get_value(&self, value: &ValueNetwork, _params: &MctsParams) -> i32 {
        const K: f32 = 400.0;
        let (win, draw, _) = value.eval(&self.board);

        let score = win + draw / 2.0;
        let cp = (-K * (1.0 / score.clamp(0.0, 1.0) - 1.0).ln()) as i32;

        #[cfg(not(feature = "datagen"))]
        {
            use consts::Piece;

            let mut mat = self.piece_count(Piece::KNIGHT) * _params.knight_value()
                + self.piece_count(Piece::BISHOP) * _params.bishop_value()
                + self.piece_count(Piece::ROOK) * _params.rook_value()
                + self.piece_count(Piece::QUEEN) * _params.queen_value();

            mat = _params.material_offset() + mat / _params.material_div1();

            cp * mat / _params.material_div2()
        }

        #[cfg(feature = "datagen")]
        cp
    }

    pub fn get_value_wdl(&self, value: &ValueNetwork, params: &MctsParams) -> f32 {
        1.0 / (1.0 + (-(self.get_value(value, params) as f32) / 400.0).exp())
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

fn perft<const ROOT: bool, const BULK: bool>(pos: &Board, depth: u8, castling: &Castling) -> u64 {
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
