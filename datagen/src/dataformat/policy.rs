use super::CompressedChessBoard;

use monty::{ChessState, Move};

#[repr(C)]
#[derive(Clone)]
pub struct PolicyData {
    pub pos: CompressedChessBoard,
    pub moves: [(u16, u16); 112],
    pub num: usize,
    pub score: f32,
    pub result: f32,
    pub best_move: u16,
}

impl PolicyData {
    pub fn new(pos: ChessState, best_move: Move, score: f32) -> Self {
        Self {
            pos: CompressedChessBoard::from(pos),
            moves: [(0, 0); 112],
            num: 0,
            score,
            result: 0.0,
            best_move: best_move.into(),
        }
    }

    pub fn push(&mut self, mov: Move, visits: i32) {
        self.moves[self.num] = (mov.into(), visits as u16);
        self.num += 1;
    }

    pub fn set_result(&mut self, result: f32) {
        self.result = result;
    }
}
