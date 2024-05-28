pub mod ataxx;
pub mod chess;
pub mod shatranj;

use crate::MctsParams;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameState {
    #[default]
    Ongoing,
    Lost(u8),
    Draw,
    Won(u8),
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

pub trait GameRep: Clone + Default + Send + Sync {
    type Move: Copy + Default + From<u16> + Into<u16> + std::fmt::Display;
    type PolicyInputs;
    const STARTPOS: &'static str;
    const MAX_MOVES: usize;

    type Policy: Send + Sync;
    type Value: Send + Sync;

    fn default_mcts_params() -> MctsParams;

    fn is_same(&self, other: &Self) -> bool;

    fn stm(&self) -> usize;

    /// For games where black goes first.
    fn tm_stm(&self) -> usize;

    fn hash(&self) -> u64 {
        unimplemented!()
    }

    fn game_state(&self) -> GameState;

    fn make_move(&mut self, mov: Self::Move);

    fn map_legal_moves<F: FnMut(Self::Move)>(&self, f: F);

    fn get_policy_feats(&self) -> Self::PolicyInputs;

    fn get_policy(&self, mov: Self::Move, feats: &Self::PolicyInputs, policy: &Self::Policy)
        -> f32;

    fn get_value(&self, value: &Self::Value) -> i32;

    fn get_value_wdl(&self, value: &Self::Value) -> f32 {
        1.0 / (1.0 + (-(self.get_value(value) as f32) / 400.0).exp())
    }

    fn from_fen(fen: &str) -> Self;

    fn conv_mov_to_str(&self, mov: Self::Move) -> String;

    fn perft(&self, depth: usize) -> u64;

    fn display(&self, policy: &Self::Policy);
}
