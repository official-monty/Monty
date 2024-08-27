use bullet::{
    format::{chess::BoardIter, ChessBoard},
    inputs,
};
use monty::Board;

#[derive(Clone, Copy, Default)]
pub struct ThreatInputs;

pub struct ThreatInputsIter {
    board_iter: BoardIter,
    threats: u64,
    defences: u64,
    flip: u8,
}

impl inputs::InputType for ThreatInputs {
    type RequiredDataType = ChessBoard;
    type FeatureIter = ThreatInputsIter;

    fn buckets(&self) -> usize {
        1
    }

    fn max_active_inputs(&self) -> usize {
        32
    }

    fn inputs(&self) -> usize {
        768 * 4
    }

    fn feature_iter(&self, pos: &Self::RequiredDataType) -> Self::FeatureIter {
        let mut bb = [0; 8];

        for (pc, sq) in pos.into_iter() {
            let bit = 1 << sq;
            bb[usize::from(pc >> 3)] ^= bit;
            bb[usize::from(2 + (pc & 7))] ^= bit;
        }

        let board = Board::from_raw(bb, false, 0, 0, 0, 1);

        let threats = board.threats_by(1);
        let defences = board.threats_by(0);

        ThreatInputsIter {
            board_iter: pos.into_iter(),
            threats,
            defences,
            flip: if pos.our_ksq() % 8 > 3 { 7 } else { 0 },
        }
    }
}

impl Iterator for ThreatInputsIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.board_iter.next().map(|(piece, square)| {
            let c = usize::from(piece & 8 > 0);
            let pc = 64 * usize::from(piece & 7);
            let sq = usize::from(square);
            let mut feat = [0, 384][c] + pc + (sq ^ usize::from(self.flip));

            if self.threats & (1 << sq) > 0 {
                feat += 768;
            }

            if self.defences & (1 << sq) > 0 {
                feat += 768 * 2;
            }

            (feat, feat)
        })
    }
}
