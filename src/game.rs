use crate::moves::{MoveList, MoveType};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameState {
    #[default]
    Ongoing,
    Lost,
    Draw,
    Won,
}

pub trait GameRep: Clone + Default + Send + Sync {
    type Policy: Send + Sync;
    type Value: Send + Sync;
    type Move: MoveType;
    //const MAX_MOVES: usize;
    const STARTPOS: &'static str;

    fn stm(&self) -> usize;

    fn game_state(&self) -> GameState;

    fn make_move(&mut self, mov: Self::Move);

    fn gen_legal_moves(&self) -> MoveList<Self::Move>;

    fn set_policies(&self, policy: &Self::Policy, moves: &mut MoveList<Self::Move>);

    fn get_value(&self, value: &Self::Value) -> f32;

    fn from_fen(fen: &str) -> Self;

    fn conv_mov_to_str(&self, mov: Self::Move) -> String;

    fn perft(&self, depth: usize) -> u64;
}
