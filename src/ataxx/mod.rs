mod board;
mod moves;
mod policy;
mod util;
mod value;

use crate::{GameRep, MoveType, UciLike};

pub use self::{board::Board, moves::Move, policy::{PolicyNetwork, SubNet, POLICY_NETWORK}};

const STARTPOS: &str = "x5o/7/7/7/7/7/o5x x 0 1";

pub struct Uai;
impl UciLike for Uai {
    type Game = Ataxx;
    const NAME: &'static str = "uai";
    const NEWGAME: &'static str = "uainewgame";
    const OK: &'static str = "uaiok";
    const FEN_STRING: &'static str = include_str!("../../resources/ataxx-fens.txt");

    fn options() {}
}

#[derive(Clone, Copy, Default)]
pub struct Ataxx {
    board: Board,
}

impl Ataxx {
    pub fn board(&self) -> &Board {
        &self.board
    }
}

impl GameRep for Ataxx {
    const STARTPOS: &'static str = STARTPOS;
    type Move = Move;

    fn stm(&self) -> usize {
        self.board.stm()
    }

    fn tm_stm(&self) -> usize {
        self.board.stm() ^ 1
    }

    fn conv_mov_to_str(&self, mov: Self::Move) -> String {
        mov.uai()
    }

    fn from_fen(fen: &str) -> Self {
        Self { board: Board::from_fen(fen) }
    }

    fn game_state(&self) -> crate::GameState {
        self.board.game_state()
    }

    fn gen_legal_moves(&self) -> crate::MoveList<Self::Move> {
        self.board.movegen()
    }

    fn get_value(&self) -> f32 {
        let out = value::ValueNetwork::eval(&self.board);

        1.0 / (1.0 + (-out as f32 / 400.0).exp())
    }

    fn set_policies(&self, moves: &mut crate::MoveList<Self::Move>) {
        let mut total = 0.0;
        let mut max = -1000.0;
        let mut floats = [0.0; 256];
        let feats = self.board.get_features();

        for (i, mov) in moves.iter_mut().enumerate() {
            floats[i] = PolicyNetwork::get(mov, &feats);
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

    fn make_move(&mut self, mov: Self::Move) {
        self.board.make(mov);
    }

    fn perft(&self, depth: usize) -> u64 {
        perft(&self.board, depth as u8)
    }
}

impl std::fmt::Display for Ataxx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut moves = self.gen_legal_moves();
        self.set_policies(&mut moves);

        let mut w = [0f32; 49];
        let mut count = [0; 49];

        for mov in moves.iter() {
            let fr = mov.from();
            let to = mov.to();

            if fr != 63 {
                w[fr] = w[fr].max(mov.policy());
                count[fr] += 1;
            }

            if to != 63 {
                w[to] = w[to].max(mov.policy());
                count[to] += 1;
            }
        }

        let bbs = self.board.bbs();

        writeln!(f, "+---------------+")?;

        for rank in (0..7).rev() {
            write!(f, "|")?;

            for file in 0..7 {
                let sq = 7 * rank + file;
                let bit = 1 << sq;

                let add = if bit & bbs[0] > 0 {
                    'x'
                } else if bit & bbs[1] > 0 {
                    'o'
                } else if bit & bbs[2] > 0 {
                    '-'
                } else {
                    '.'
                };

                if count[sq] > 0 {
                    let g = (255.0 * (2.0 * w[sq]).min(1.0)) as u8;
                    let r = 255 - g;
                    write!(f, " \x1b[38;2;{r};{g};0m{add}\x1b[0m")?;
                } else {
                    write!(f, " \x1b[34m{add}\x1b[0m")?;
                }
            }

            writeln!(f, " |")?;
        }

        writeln!(f, "+---------------+")?;

        Ok(())
    }
}

fn perft(board: &Board, depth: u8) -> u64 {
    if depth == 1 {
        return board.movegen_bulk(false);
    }

    let moves = board.movegen();
    let mut nodes = 0;

    for &mov in moves.iter() {
        let mut new = *board;

        if mov.is_pass() {
            continue;
        }

        new.make(mov);
        nodes += perft(&new, depth - 1);
    }

    nodes
}
