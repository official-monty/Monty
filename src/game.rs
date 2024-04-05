use goober::SparseVector;

use crate::MctsParams;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameState {
    #[default]
    Ongoing,
    Lost,
    Draw,
    Won,
}

pub trait GameRep: Clone + Default + Send + Sync + std::fmt::Display {
    type Move: Copy + Default + From<u16> + Into<u16>;
    const STARTPOS: &'static str;
    const MAX_MOVES: usize;

    fn default_mcts_params() -> MctsParams;

    fn is_same(&self, other: &Self) -> bool;

    fn stm(&self) -> usize;

    /// For games where black goes first.
    fn tm_stm(&self) -> usize;

    fn game_state(&self) -> GameState;

    fn make_move(&mut self, mov: Self::Move);

    fn map_legal_moves<F: FnMut(Self::Move)>(&self, f: F);

    fn get_policy_feats(&self) -> SparseVector;

    fn get_policy(&self, mov: Self::Move, feats: &SparseVector) -> f32;

    fn get_value(&self) -> f32;

    fn from_fen(fen: &str) -> Self;

    fn conv_mov_to_str(&self, mov: Self::Move) -> String;

    fn perft(&self, depth: usize) -> u64;
}
