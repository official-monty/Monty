use monty_core::{Move, Position};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct TrainingMove {
    mov: u16,
    visits: u16,
}

impl TrainingMove {
    pub fn new(mov: &Move, visits: i32) -> Self {
        let from = u16::from(mov.from()) << 10;
        let to = u16::from(mov.to()) << 4;
        Self {
            mov: from | to | u16::from(mov.flag()),
            visits: visits as u16,
        }
    }

    pub fn mov(&self, pos: &Position) -> Move {
        let from = (self.mov >> 10) as u8;
        let to = (self.mov >> 4) as u8 & 0b111111;
        let flag = self.mov as u8 & 0b1111;
        let moved = pos.get_pc(1 << from) as u8;

        Move::new(from, to, flag, moved)
    }

    pub fn visits(&self) -> i32 {
        i32::from(self.visits)
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct TrainingMoveList {
    list: [TrainingMove; 106],
    len: usize,
}

impl Default for TrainingMoveList {
    fn default() -> Self {
        Self {
            list: [TrainingMove::default(); 106],
            len: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct TrainingPosition {
    position: Position,
    moves: TrainingMoveList,
}

impl std::fmt::Debug for TrainingPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "FEN: {}", self.board().to_fen())?;
        for mov in self.moves() {
            writeln!(f, "{}: {}", mov.mov(self.board()).to_uci(), mov.visits)?;
        }
        Ok(())
    }
}

impl TrainingPosition {
    pub fn new(position: Position) -> Self {
        Self {
            position,
            moves: TrainingMoveList::default(),
        }
    }

    pub fn push(&mut self, mov: &Move, visits: i32) {
        self.moves.list[self.moves.len] = TrainingMove::new(mov, visits);
        self.moves.len += 1;
    }

    pub fn num_moves(&self) -> usize {
        self.moves.len
    }

    pub fn board(&self) -> &Position {
        &self.position
    }

    pub fn moves(&self) -> &[TrainingMove] {
        &self.moves.list[..self.moves.len]
    }
}
