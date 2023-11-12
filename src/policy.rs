use crate::{consts::Flag, moves::Move, params::TunableParams};

pub fn get_policy(mov: &Move, params: &TunableParams) -> f64 {
    if mov.flag() & Flag::CAP > 0 {
        params.cap()
    } else {
        0.0
    }
}