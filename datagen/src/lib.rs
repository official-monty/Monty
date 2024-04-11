pub mod impls;
mod rng;
mod thread;

pub use rng::Rand;
pub use thread::{write, DatagenThread};

use monty::GameRep;

#[repr(C)]
pub struct PolicyData<T: DatagenSupport, const MAX: usize> {
    pub pos: T::CompressedBoard,
    pub moves: [(u16, u16); MAX],
    pub num: usize,
    pub score: f32,
    pub result: f32,
    pub best_move: u16,
}

impl<T: DatagenSupport, const MAX: usize> PolicyData<T, MAX> {
    pub fn new(pos: T, best_move: T::Move, score: f32) -> Self {
        Self {
            pos: T::CompressedBoard::from(pos),
            moves: [(0, 0); MAX],
            num: 0,
            score,
            result: 0.0,
            best_move: best_move.into(),
        }
    }

    pub fn push(&mut self, mov: T::Move, visits: i32) {
        self.moves[self.num] = (mov.into(), visits as u16);
        self.num += 1;
    }

    pub fn set_result(&mut self, result: f32) {
        self.result = result;
    }
}

pub trait DatagenSupport: GameRep {
    type CompressedBoard: Copy + From<Self>;
    type Binpack: BinpackType<Self>;
}

pub trait BinpackType<T: GameRep>: Sized {
    fn new(pos: T) -> Self;

    fn push(&mut self, stm: usize, mov: T::Move, score: f32);

    fn set_result(&mut self, result: f32);

    fn serialise_into(&self, writer: &mut impl std::io::Write) -> std::io::Result<()>;

    fn deserialise_from(reader: &mut impl std::io::BufRead, buffer: Vec<(u16, i16)>) -> std::io::Result<Self>;
}

pub fn to_slice_with_lifetime<T, U>(slice: &[T]) -> &[U] {
    let src_size = std::mem::size_of_val(slice);
    let tgt_size = std::mem::size_of::<U>();

    assert!(
        src_size % tgt_size == 0,
        "Target type size does not divide slice size!"
    );

    let len = src_size / tgt_size;
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), len) }
}
