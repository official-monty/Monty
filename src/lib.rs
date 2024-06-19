mod chess;
mod mcts;
mod networks;
mod tree;
mod uci;

pub use chess::{Board, Castling, ChessState, GameState, Move};
pub use mcts::{Limits, MctsParams, Searcher};
pub use networks::{
    PolicyFileDefaultName, PolicyNetwork, SubNet, ValueFileDefaultName, ValueNetwork,
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
