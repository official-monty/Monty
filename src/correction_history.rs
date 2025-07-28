use std::sync::atomic::{AtomicI32, Ordering};

use crate::chess::Board;

/// Parameters for correction history.
const CORRHIST_SIZE: usize = 1 << 16; // 64k entries
const CORRHIST_WEIGHT_SCALE: i32 = 64;

pub struct CorrectionHistory {
    table: Vec<AtomicI32>,
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrectionHistory {
    /// Create a new correction history table
    pub fn new() -> Self {
        Self {
            table: (0..CORRHIST_SIZE).map(|_| AtomicI32::new(0)).collect(),
        }
    }

    #[inline]
    fn index(&self, board: &Board) -> usize {
        board.hash() as usize % CORRHIST_SIZE
    }

    /// Return the current correction in centipawns for the board
    pub fn get(&self, board: &Board) -> i32 {
        self.table[self.index(board)].load(Ordering::Relaxed)
    }

    /// Adjust a static evaluation with correction history
    pub fn apply(&self, board: &Board, eval: i32) -> i32 {
        eval + self.get(board)
    }

    /// Update correction history using depth and evaluation difference
    pub fn update(&self, board: &Board, diff: i32) {
        let idx = self.index(board);
        let entry = self.table[idx].load(Ordering::Relaxed);
        let value = (entry * (CORRHIST_WEIGHT_SCALE - 1) + diff) / CORRHIST_WEIGHT_SCALE;
        self.table[idx].store(value, Ordering::Relaxed);
    }
}
