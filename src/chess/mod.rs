mod attacks;
mod board;
pub mod consts;
mod frc;
mod moves;
mod policy;
mod qsearch;
mod value;

use crate::{
    comm::UciLike,
    game::{GameRep, GameState},
    moves::{MoveList, MoveType},
};

use self::{frc::Castling, moves::Move, qsearch::quiesce};

pub use self::{
    board::Board,
    policy::{PolicyNetwork, SubNet, POLICY_NETWORK},
    value::{ValueNetwork, NNUE},
};

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct Uci;
impl UciLike for Uci {
    const NAME: &'static str = "uci";
    const NEWGAME: &'static str = "ucinewgame";
    const OK: &'static str = "uciok";
    const FEN_STRING: &'static str = include_str!("../../resources/chess-fens.txt");

    type Game = Chess;

    fn options() {
        println!("option name UCI_Chess960 type check default false");
    }
}

#[derive(Clone)]
pub struct Chess {
    board: Board,
    castling: Castling,
    stack: Vec<u64>,
}

impl Default for Chess {
    fn default() -> Self {
        let mut castling = Castling::default();
        let board = Board::parse_fen(STARTPOS, &mut castling);

        Self {
            board,
            castling,
            stack: Vec::new(),
        }
    }
}

impl Chess {
    pub fn bbs(&self) -> [u64; 8] {
        self.board.bbs()
    }

    pub fn board(&self) -> Board {
        self.board
    }
}

impl GameRep for Chess {
    type Move = Move;

    const STARTPOS: &'static str = STARTPOS;

    fn conv_mov_to_str(&self, mov: Self::Move) -> String {
        mov.to_uci(&self.castling)
    }

    fn from_fen(fen: &str) -> Self {
        let mut castling = Castling::default();
        let board = Board::parse_fen(fen, &mut castling);

        Self {
            board,
            castling,
            stack: Vec::new(),
        }
    }

    fn gen_legal_moves(&self) -> MoveList<Move> {
        self.board.gen::<true>(&self.castling)
    }

    fn game_state(&self) -> GameState {
        let moves = self.gen_legal_moves();
        self.board.game_state(&moves, &self.stack)
    }

    fn make_move(&mut self, mov: Self::Move) {
        self.stack.push(self.board.hash());
        self.board.make(mov, None, &self.castling);
    }

    fn stm(&self) -> usize {
        self.board.stm()
    }

    fn tm_stm(&self) -> usize {
        self.stm()
    }

    fn get_value(&self) -> f32 {
        let accs = self.board.get_accs();
        let qs = quiesce(&self.board, &self.castling, &accs, -30_000, 30_000);
        1.0 / (1.0 + (-(qs as f32) / (400.0)).exp())
    }

    fn set_policies(&self, moves: &mut MoveList<Move>) {
        let mut total = 0.0;
        let mut max = -1000.0;
        let mut floats = [0.0; 256];
        let feats = self.board.get_features();

        for (i, mov) in moves.iter_mut().enumerate() {
            floats[i] = PolicyNetwork::get(mov, &self.board, &POLICY_NETWORK, &feats);
            if floats[i] > max {
                max = floats[i];
            }
        }

        for (i, _) in moves.iter_mut().enumerate() {
            floats[i] = (floats[i] - max).exp();
            total += floats[i];
        }

        for (i, mov) in moves.iter_mut().enumerate() {
            mov.set_policy(floats[i] / total);
        }
    }

    fn perft(&self, depth: usize) -> u64 {
        perft::<true, true>(&self.board, depth as u8, &self.castling)
    }
}

fn perft<const ROOT: bool, const BULK: bool>(pos: &Board, depth: u8, castling: &Castling) -> u64 {
    let moves = pos.gen::<true>(castling);

    if BULK && !ROOT && depth == 1 {
        return moves.len() as u64;
    }

    let mut positions = 0;
    let leaf = depth == 1;

    for m_idx in 0..moves.len() {
        let mut tmp = *pos;
        tmp.make(moves[m_idx], None, castling);

        let num = if !BULK && leaf {
            1
        } else {
            perft::<false, BULK>(&tmp, depth - 1, castling)
        };
        positions += num;

        if ROOT {
            println!("{}: {num}", moves[m_idx].to_uci(castling));
        }
    }

    positions
}
