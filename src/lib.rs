mod comm;
mod games;
mod mcts;
mod params;
mod tree;

#[cfg(any(feature = "ataxx", feature = "shatranj"))]
mod value;

pub use comm::UciLike;
pub use games::{GameRep, GameState};
pub use mcts::{Limits, Searcher};
pub use params::MctsParams;
pub use tree::Tree;

#[cfg(feature = "ataxx")]
pub use games::ataxx;

#[cfg(not(feature = "ataxx"))]
#[cfg(not(feature = "shatranj"))]
pub use games::chess;

#[cfg(feature = "shatranj")]
pub use games::shatranj;

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
