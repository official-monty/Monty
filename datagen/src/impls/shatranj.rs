use monty::shatranj::{Board, Move, Shatranj};

use crate::{BinpackType, DatagenSupport};

impl DatagenSupport for Shatranj {
    type CompressedBoard = CompressedShatranjBoard;
    type Binpack = Binpack;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompressedShatranjBoard {
    bbs: [u64; 4],
    stm: bool,
    halfm: u8,
}

impl From<Shatranj> for CompressedShatranjBoard {
    fn from(value: Shatranj) -> Self {
        Self::from(value.board())
    }
}

impl From<Board> for CompressedShatranjBoard {
    fn from(board: Board) -> Self {
        let bbs = board.bbs();

        Self {
            bbs: [
                bbs[1],
                bbs[5] ^ bbs[6] ^ bbs[7],
                bbs[3] ^ bbs[4] ^ bbs[7],
                bbs[2] ^ bbs[4] ^ bbs[6],
            ],
            stm: board.stm() > 0,
            halfm: board.halfm(),
        }
    }
}

impl From<CompressedShatranjBoard> for Board {
    fn from(value: CompressedShatranjBoard) -> Self {
        let qbbs = value.bbs;

        let mut bbs = [0; 8];

        let blc = qbbs[0];
        let rqk = qbbs[1];
        let nbk = qbbs[2];
        let pbq = qbbs[3];

        let occ = rqk | nbk | pbq;
        let pnb = occ ^ qbbs[1];
        let prq = occ ^ qbbs[2];
        let nrk = occ ^ qbbs[3];

        bbs[0] = occ ^ blc;
        bbs[1] = blc;
        bbs[2] = pnb & prq;
        bbs[3] = pnb & nrk;
        bbs[4] = pnb & nbk & pbq;
        bbs[5] = prq & nrk;
        bbs[6] = pbq & prq & rqk;
        bbs[7] = nbk & rqk;

        Board::from_raw(bbs, value.stm, value.halfm)
    }
}

impl CompressedShatranjBoard {
    fn as_bytes(self) -> [u8; std::mem::size_of::<CompressedShatranjBoard>()] {
        unsafe { std::mem::transmute(self) }
    }

    fn from_bytes(bytes: [u8; std::mem::size_of::<CompressedShatranjBoard>()]) -> Self {
        unsafe { std::mem::transmute(bytes) }
    }

    pub fn stm(&self) -> bool {
        self.stm
    }
}

pub struct Binpack {
    startpos: CompressedShatranjBoard,
    result: u8,
    moves: Vec<(u16, i16)>,
}

impl BinpackType<Shatranj> for Binpack {
    fn new(pos: Shatranj) -> Self {
        Self {
            startpos: pos.into(),
            result: 3,
            moves: Vec::new(),
        }
    }

    fn set_result(&mut self, result: f32) {
        self.result = (2.0 * result) as u8;
    }

    fn push(&mut self, stm: usize, best_move: Move, mut score: f32) {
        if stm == 1 {
            score = 1.0 - score;
        }

        let score = -(400.0 * (1.0 / score - 1.0).ln()) as i16;

        self.moves.push((best_move.into(), score));
    }

    fn serialise_into(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        writer.write_all(&self.startpos.as_bytes())?;
        writer.write_all(&[self.result])?;

        for (mov, score) in &self.moves {
            writer.write_all(&mov.to_le_bytes())?;
            writer.write_all(&score.to_le_bytes())?;
        }

        writer.write_all(&[0; 4])?;
        Ok(())
    }

    fn deserialise_from(
        reader: &mut impl std::io::BufRead,
        buffer: Vec<(u16, i16)>,
    ) -> std::io::Result<Self> {
        let mut startpos = [0; std::mem::size_of::<CompressedShatranjBoard>()];
        reader.read_exact(&mut startpos)?;
        let startpos = CompressedShatranjBoard::from_bytes(startpos);

        let mut result = [0];
        reader.read_exact(&mut result)?;
        let result = result[0];

        let mut moves = buffer;
        moves.clear();

        loop {
            let mut buf = [0; 4];
            reader.read_exact(&mut buf)?;

            if buf == [0; 4] {
                break;
            }

            let mov = u16::from_le_bytes([buf[0], buf[1]]);
            let score = i16::from_le_bytes([buf[2], buf[3]]);

            moves.push((mov, score));
        }

        Ok(Self {
            startpos,
            result,
            moves,
        })
    }
}

impl Binpack {
    pub fn deserialise_map<F>(reader: &mut impl std::io::BufRead, mut f: F) -> std::io::Result<()>
    where
        F: FnMut(&mut Board, Move, i16, f32),
    {
        let mut startpos = [0; std::mem::size_of::<CompressedShatranjBoard>()];
        reader.read_exact(&mut startpos)?;
        let startpos = CompressedShatranjBoard::from_bytes(startpos);

        let mut result = [0];
        reader.read_exact(&mut result)?;
        let result = f32::from(result[0]) / 2.0;

        let mut board = Board::from(startpos);

        loop {
            let mut buf = [0; 4];
            reader.read_exact(&mut buf)?;

            if buf == [0; 4] {
                break;
            }

            let mov = u16::from_le_bytes([buf[0], buf[1]]);
            let score = i16::from_le_bytes([buf[2], buf[3]]);

            f(&mut board, mov.into(), score, result);
        }

        Ok(())
    }
}
