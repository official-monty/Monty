use monty::{Board, ChessState};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompressedChessBoard {
    bbs: [u64; 4],
    stm: bool,
    enp_sq: u8,
    rights: u8,
    halfm: u8,
    rook_files: [[u8; 2]; 2],
}

impl From<ChessState> for CompressedChessBoard {
    fn from(value: ChessState) -> Self {
        let mut ret = Self::from(value.board());

        ret.rook_files = value.castling().rook_files();

        ret
    }
}

impl From<Board> for CompressedChessBoard {
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
            enp_sq: board.enp_sq(),
            rights: board.rights(),
            halfm: board.halfm(),
            rook_files: [[0; 2]; 2],
        }
    }
}

impl From<CompressedChessBoard> for Board {
    fn from(value: CompressedChessBoard) -> Self {
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

        Board::from_raw(bbs, value.stm, value.enp_sq, value.rights, value.halfm, 1)
    }
}

impl CompressedChessBoard {
    pub fn as_bytes(self) -> [u8; std::mem::size_of::<CompressedChessBoard>()] {
        unsafe { std::mem::transmute(self) }
    }

    pub fn from_bytes(bytes: [u8; std::mem::size_of::<CompressedChessBoard>()]) -> Self {
        unsafe { std::mem::transmute(bytes) }
    }

    pub fn stm(&self) -> bool {
        self.stm
    }

    pub fn rook_files(&self) -> [[u8; 2]; 2] {
        self.rook_files
    }
}
