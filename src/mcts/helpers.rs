use crate::{mcts::MctsParams, tree::{ActionStats, Edge}};

pub struct SearchHelpers;

impl SearchHelpers {
    /// CPUCT
    ///
    /// Larger value implies more exploration.
    pub fn get_cpuct(params: &MctsParams, node_stats: &ActionStats, is_root: bool) -> f32 {
        // baseline CPUCT value
        let mut cpuct = if is_root {
            params.root_cpuct()
        } else {
            params.cpuct()
        };

        // scale CPUCT as visits increase
        let scale = params.cpuct_visits_scale() * 128.0;
        cpuct *= 1.0 + ((node_stats.visits() as f32 + scale) / scale).ln();

        // scale CPUCT with variance of Q
        if node_stats.visits() > 1 {
            let frac = node_stats.var().sqrt() / params.cpuct_var_scale();
            cpuct *= 1.0 + params.cpuct_var_weight() * (frac - 1.0);
        }

        cpuct
    }

    /// Exploration Scaling
    ///
    /// Larger value implies more exploration.
    pub fn get_explore_scaling(params: &MctsParams, node_stats: &ActionStats) -> f32 {
        (params.expl_tau() * (node_stats.visits().max(1) as f32).ln()).exp()
    }

    /// First Play Urgency
    ///
    /// #### Note
    /// Must return a value in [0, 1].
    pub fn get_fpu(node_stats: &ActionStats) -> f32 {
        1.0 - node_stats.q()
    }

    /// Get a predicted win probability for an action
    ///
    /// #### Note
    /// Must return a value in [0, 1].
    pub fn get_action_value(action: &Edge, fpu: f32) -> f32 {
        if action.visits() == 0 {
            fpu
        } else {
            action.q()
        }
    }

    /// Calculates the maximum allowed time usage for a search
    ///
    /// #### Note
    /// This will be overriden by a `go movetime` command,
    /// and a move overhead will be applied to this, so no
    /// need for it here.
    pub fn get_time(
        time: u64,
        increment: Option<u64>,
        ply: u32,
        movestogo: Option<u64>,
        params: &MctsParams,
    ) -> (u128, u128) {
        if let Some(mtg) = movestogo {
            // Cyclic time control (x moves in y seconds)
            let max_time = (time as f64 / (mtg as f64).clamp(1.0, 30.0)) as u128;
            (max_time, max_time)
        } else {
            // Increment time control (x seconds + y increment)
            let inc = increment.unwrap_or(0);
            let mtg = params.tm_mtg() as u64;

            let time_left = (time + inc * (mtg - 1) - 10 * (2 + mtg)).max(1) as f64;
            let log_time = (time_left / 1000.0).log10();

            let opt_constant = (params.tm_opt_value1() / 100.0
                + params.tm_opt_value2() / 1000.0 * log_time)
                .min(params.tm_opt_value3() / 100.0);
            let opt_scale = (params.tm_optscale_value1() / 100.0
                + (ply as f64 + params.tm_optscale_value2()).powf(params.tm_optscale_value3())
                    * opt_constant)
                .min(params.tm_optscale_value4() * time as f64 / time_left);

            let max_constant = (params.tm_max_value1() + params.tm_max_value2() * log_time)
                .max(params.tm_max_value3());
            let max_scale = (max_constant + ply as f64 / params.tm_maxscale_value1())
                .min(params.tm_maxscale_value2());

            // More time at the start of the game
            let bonus_ply = params.tm_bonus_ply();
            let bonus = if ply < bonus_ply as u32 {
                1.0 + (bonus_ply - ply as f64).log10() * params.tm_bonus_value1()
            } else {
                1.0
            };

            let opt_time = (opt_scale * bonus * time_left) as u128;
            let max_time =
                (max_scale * opt_time as f64).min(time as f64 * params.tm_max_time()) as u128;

            (opt_time, max_time)
        }
    }
}
