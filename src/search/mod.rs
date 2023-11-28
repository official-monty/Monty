pub mod mcts;
pub mod params;
pub mod policy;
pub mod qsearch;
pub mod value;

use params::TunableParams;

pub fn cp_wdl(score: i32, params: &TunableParams) -> f64 {
    1.0 / (1.0 + (-f64::from(score) / (100.0 * params.scale())).exp())
}
