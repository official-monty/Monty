use crate::{mcts::MctsParams, tree::Edge};

pub struct SearchHelpers;

impl SearchHelpers {
    pub fn get_cpuct(params: &MctsParams, _: &Edge, is_root: bool) -> f32 {
        // baseline CPUCT value
        if is_root {
            params.root_cpuct()
        } else {
            params.cpuct()
        }
    }

    pub fn get_explore_scaling(_: &MctsParams, parent: &Edge) -> f32 {
        (parent.visits().max(1) as f32).sqrt()
    }

    pub fn get_fpu(parent: &Edge) -> f32 {
        1.0 - parent.q()
    }

    pub fn get_action_value(action: &Edge, fpu: f32) -> f32 {
        if action.visits() == 0 {
            fpu
        } else {
            action.q()
        }
    }
}
