mod board;
mod moves;
mod policy;
mod util;

use goober::SparseVector;

use crate::{
    value::{ValueFeatureMap, ValueNetwork},
    GameRep, MctsParams, UciLike,
};

pub use self::{
    board::Board,
    moves::Move,
    policy::{PolicyNetwork, SubNet},
};

const STARTPOS: &str = "x5o/7/7/7/7/7/o5x x 0 1";

impl ValueFeatureMap for Board {
    fn value_feature_map<F: FnMut(usize)>(&self, f: F) {
        self.value_features_map(f);
    }
}

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

impl From<Ataxx> for Board {
    fn from(value: Ataxx) -> Self {
        value.board
    }
}

impl GameRep for Ataxx {
    const STARTPOS: &'static str = STARTPOS;
    const MAX_MOVES: usize = 256;
    type Move = Move;
    type PolicyInputs = goober::SparseVector;

    type Policy = PolicyNetwork;
    type Value = ValueNetwork<2916, 256>;

    fn default_mcts_params() -> MctsParams {
        MctsParams::default()
    }

    fn is_same(&self, other: &Self) -> bool {
        self.board == other.board
    }

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
        Self {
            board: Board::from_fen(fen),
        }
    }

    fn game_state(&self) -> crate::GameState {
        self.board.game_state()
    }

    fn map_legal_moves<F: FnMut(Self::Move)>(&self, f: F) {
        self.board.map_legal_moves(f);
    }

    fn get_value(&self, value: &Self::Value) -> i32 {
        value.eval(&self.board)
    }

    fn get_policy_feats(&self) -> SparseVector {
        self.board.get_features()
    }

    fn get_policy(&self, mov: Self::Move, feats: &SparseVector, policy: &Self::Policy) -> f32 {
        policy.get(&mov, feats)
    }

    fn make_move(&mut self, mov: Self::Move) {
        self.board.make(mov);
    }

    fn perft(&self, depth: usize) -> u64 {
        perft(&self.board, depth as u8)
    }

    fn hash(&self) -> u64 {
        use self::util::ZVALS;

        let mut hash = 0;

        let mut boys = self.board.boys();
        while boys > 0 {
            let sq = boys.trailing_zeros() as usize;
            boys &= boys - 1;

            hash ^= ZVALS[0][sq];
        }

        let mut opps = self.board.opps();
        while opps > 0 {
            let sq = opps.trailing_zeros() as usize;
            opps &= opps - 1;

            hash ^= ZVALS[1][sq];
        }

        hash
    }

    fn display(&self, policy: &Self::Policy) {
        let feats = self.get_policy_feats();
        let mut moves = Vec::new();
        let mut max = f32::NEG_INFINITY;
        self.map_legal_moves(|mov| {
            let policy = self.get_policy(mov, &feats, policy);
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

        let mut w = [0f32; 49];
        let mut count = [0; 49];

        for &(mov, policy) in moves.iter() {
            let fr = mov.from();
            let to = mov.to();

            if fr != 63 {
                w[fr] = w[fr].max(policy);
                count[fr] += 1;
            }

            if to != 63 {
                w[to] = w[to].max(policy);
                count[to] += 1;
            }
        }

        let bbs = self.board.bbs();

        println!("+---------------+");

        for rank in (0..7).rev() {
            print!("|");

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
                    print!(" \x1b[38;2;{r};{g};0m{add}\x1b[0m");
                } else {
                    print!(" \x1b[34m{add}\x1b[0m");
                }
            }

            println!(" |");
        }

        println!("+---------------+");
    }
}

fn perft(board: &Board, depth: u8) -> u64 {
    if depth == 1 {
        return board.movegen_bulk(false);
    }

    let mut nodes = 0;

    board.map_legal_moves(|mov| {
        let mut new = *board;

        if !mov.is_pass() {
            new.make(mov);
            nodes += perft(&new, depth - 1);
        }
    });

    nodes
}
