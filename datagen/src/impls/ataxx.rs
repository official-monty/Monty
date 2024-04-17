use monty::ataxx::{Ataxx, Board, Move};

use crate::{BinpackType, DatagenSupport};

impl DatagenSupport for Ataxx {
    type CompressedBoard = Board;
    type Binpack = ();
}

impl BinpackType<Ataxx> for <Ataxx as DatagenSupport>::Binpack {
    fn new(_: Ataxx) -> Self {}

    fn push(&mut self, _: usize, _: Move, _: f32) {}

    fn deserialise_from(_: &mut impl std::io::BufRead, _: Vec<(u16, i16)>) -> std::io::Result<Self> {
        Ok(())
    }

    fn serialise_into(&self, _: &mut impl std::io::Write) -> std::io::Result<()> {
        Ok(())
    }

    fn set_result(&mut self, _: f32) {}
}
