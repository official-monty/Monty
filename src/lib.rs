mod chess;
mod mcts;
mod networks;
mod tree;
mod uci;

pub use chess::{Board, Castling, ChessState, GameState, Move};
pub use mcts::{Limits, MctsParams, Searcher};
pub use networks::{
    PolicyFileDefaultName, PolicyNetwork, UnquantisedPolicyNetwork, UnquantisedValueNetwork,
    ValueFileDefaultName, ValueNetwork,
};
pub use tree::Tree;
pub use uci::Uci;

// Macro for calculating tables (until const fn pointers are stable).
#[macro_export]
macro_rules! init {
    (|$sq:ident, $size:literal | $($rest:tt)+) => {{
        let mut $sq = 0;
        let mut res = [{$($rest)+}; $size];
        while $sq < $size {
            res[$sq] = {$($rest)+};
            $sq += 1;
        }
        res
    }};
}

#[macro_export]
macro_rules! pop_lsb {
    ($idx:ident, $x:expr) => {
        let $idx = $x.trailing_zeros() as u16;
        $x &= $x - 1
    };
}

/// # Safety
/// Object must be valid if fully zeroed.
pub unsafe fn boxed_and_zeroed<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(ptr.cast())
    }
}

/// # Safety
/// Only to be used internally.
pub unsafe fn read_into_struct_unchecked<T>(path: &str) -> Box<T> {
    use std::io::Read;

    let mut f = std::fs::File::open(path).unwrap();
    let mut x: Box<T> = boxed_and_zeroed();

    let size = std::mem::size_of::<T>();

    let file_size = f.metadata().unwrap().len();

    assert_eq!(file_size as usize, size);

    unsafe {
        let slice = std::slice::from_raw_parts_mut(x.as_mut() as *mut T as *mut u8, size);
        f.read_exact(slice).unwrap();
    }

    x
}
