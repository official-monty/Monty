use bulletformat::ChessBoard;
use monty::{
    shatranj::{Board, Shatranj},
    GameRep,
};

use crate::{DatagenSupport, PolicyFormat};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ShatranjMoveInfo {
    pub mov: u16,
    pub visits: i16,
}

#[repr(C)]
pub struct ShatranjPolicyData {
    pub board: Board,
    pub result: f32,
    pub score: f32,
    // could go lower limit but for consistency
    pub moves: [ShatranjMoveInfo; 104],
    pub num: usize,
}

impl PolicyFormat<Shatranj> for ShatranjPolicyData {
    const MAX_MOVES: usize = 104;

    fn push(&mut self, mov: <Shatranj as GameRep>::Move, visits: i16) {
        self.moves[self.num] = ShatranjMoveInfo {
            mov: u16::from(mov),
            visits,
        };

        self.num += 1;
    }

    fn set_result(&mut self, result: f32) {
        self.result = result;
    }
}

impl DatagenSupport for Shatranj {
    type MoveInfo = ShatranjMoveInfo;
    type ValueData = ChessBoard;
    type PolicyData = ShatranjPolicyData;

    fn into_policy(pos: &Self, mut score: f32) -> Self::PolicyData {
        if pos.stm() == 1 {
            score = 1.0 - score;
        }

        ShatranjPolicyData {
            board: pos.board(),
            score,
            result: 0.5,
            moves: [ShatranjMoveInfo::default(); 104],
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
