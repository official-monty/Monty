use bulletformat::AtaxxBoard;
use monty::{
    ataxx::{Ataxx, Board, Move},
    GameRep,
};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AtaxxMoveInfo {
    pub from: u8,
    pub to: u8,
    pub visits: i16,
}

#[repr(C)]
pub struct AtaxxPolicyData {
    pub board: Board,
    pub result: f32,
    pub score: f32,
    pub moves: [AtaxxMoveInfo; 116],
    pub num: usize,
}

impl PolicyFormat<Ataxx> for AtaxxPolicyData {
    const MAX_MOVES: usize = 116;

    fn push(&mut self, mov: Move, visits: i16) {
        self.moves[self.num] = AtaxxMoveInfo {
            from: mov.from() as u8,
            to: mov.to() as u8,
            visits,
        };
        self.num += 1;
    }

    fn set_result(&mut self, result: f32) {
        self.result = result;
    }
}

use crate::{DatagenSupport, PolicyFormat};

impl DatagenSupport for Ataxx {
    type MoveInfo = AtaxxMoveInfo;
    type PolicyData = AtaxxPolicyData;
    type ValueData = AtaxxBoard;

    fn into_policy(pos: &Self, mut score: f32) -> Self::PolicyData {
        if pos.stm() > 0 {
            score = 1.0 - score;
        }

        AtaxxPolicyData {
            board: *pos.board(),
            result: 0.5,
            score,
            moves: [AtaxxMoveInfo::default(); 116],
            num: 0,
        }
    }

    fn into_value(pos: &Self, score: f32) -> Self::ValueData {
        let board = pos.board();
        let stm = board.stm();
        let bbs = board.bbs();

        let mut score = -(400.0 * (1.0 / score - 1.0).ln()) as i16;

        if pos.stm() == 1 {
            score = -score;
        }

        AtaxxBoard::from_raw(bbs, score, 0.5, stm > 0, board.fullm(), board.halfm())
    }
}
