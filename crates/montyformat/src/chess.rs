pub mod attacks;
pub mod consts;
pub mod frc;
pub mod moves;
pub mod position;

pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub use attacks::Attacks;
pub use consts::{Flag, Piece, Right, Side};
pub use frc::Castling;
pub use moves::Move;
pub use position::Position;

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

pub fn perft<const REPORT: bool>(pos: &Position, castling: &Castling, depth: u8) -> u64 {
    if depth == 1 {
        let mut count = 0;
        pos.map_legal_moves(castling, |_| count += 1);
        return count;
    }

    let mut count = 0;

    pos.map_legal_moves(castling, |mov| {
        let mut new = *pos;
        new.make(mov, castling);

        let sub_count = perft::<false>(&new, castling, depth - 1);

        if REPORT {
            println!("{}: {sub_count}", mov.to_uci(castling));
        }

        count += sub_count;
    });

    count
}
