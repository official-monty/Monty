use crate::{consts::Flag, moves::Move, params::TunableParams};

pub fn get_policy(mov: &Move, threats: u64, params: &TunableParams) -> f64 {
    if mov.flag() & Flag::CAP > 0 {
        params.cap()
    } else if threats & (1 << mov.to()) == 0 {
        params.pawn_threat()
    } else {
        0.0
    }
}