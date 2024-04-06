use bulletformat::ChessBoard;
use monty::{
    chess::{Board, Chess},
    GameRep,
};

use crate::{DatagenSupport, PolicyFormat};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ChessMoveInfo {
    pub mov: u16,
    pub visits: i16,
}

#[repr(C)]
pub struct ChessPolicyData {
    pub board: Board,
    pub result: f32,
    pub score: f32,
    pub moves: [ChessMoveInfo; 104],
    pub num: usize,
}

impl PolicyFormat<Chess> for ChessPolicyData {
    const MAX_MOVES: usize = 104;

    fn push(&mut self, mov: <Chess as GameRep>::Move, visits: i16) {
        self.moves[self.num] = ChessMoveInfo {
            mov: u16::from(mov),
            visits,
        };

        self.num += 1;
    }

    fn set_result(&mut self, result: f32) {
        self.result = result;
    }
}

impl DatagenSupport for Chess {
    type MoveInfo = ChessMoveInfo;
    type ValueData = ChessBoard;
    type PolicyData = ChessPolicyData;

    fn into_policy(pos: &Self, mut score: f32) -> Self::PolicyData {
        if pos.stm() == 1 {
            score = 1.0 - score;
        }

        ChessPolicyData {
            board: pos.board(),
            score,
            result: 0.5,
            moves: [ChessMoveInfo::default(); 104],
            num: 0,
        }
    }

    fn into_value(pos: &Self, mut score: f32) -> Self::ValueData {
        let stm = pos.stm();
        let bbs = pos.bbs();

        if pos.stm() == 1 {
            score = 1.0 - score;
        }

        let score = -(400.0 * (1.0 / score - 1.0).ln()) as i16;

        ChessBoard::from_raw(bbs, stm, score, 0.5).unwrap()
    }
}
