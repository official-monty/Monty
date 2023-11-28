pub mod mcts;
pub mod params;
pub mod policy;
pub mod qsearch;
pub mod value;

use params::TunableParams;

pub fn cp_wdl(score: i32, params: &TunableParams) -> f32 {
    1.0 / (1.0 + (-(score as f32) / (100.0 * params.scale())).exp())
}
