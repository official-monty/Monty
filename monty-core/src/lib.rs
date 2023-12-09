mod attacks;
mod consts;
mod moves;
mod policy;
mod position;
mod value;

pub use consts::{Flag, Piece, Side};
pub use moves::{Move, MoveList};
pub use policy::{PolicyNetwork, POLICY_NETWORK, SubNet};
pub use position::{perft, GameState, Position};
pub use value::Accumulator;

pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn cp_wdl(score: i32) -> f32 {
    1.0 / (1.0 + (-(score as f32) / (400.0)).exp())
}
