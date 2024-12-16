use std::time::Instant;

use crate::{
    mcts::{MctsParams, Searcher},
    tree::Node,
};

pub struct SearchHelpers;

impl SearchHelpers {
    /// CPUCT
    ///
    /// Larger value implies more exploration.
    pub fn get_cpuct(params: &MctsParams, node: &Node, is_root: bool) -> f32 {
        // baseline CPUCT value
        let mut cpuct = if is_root {
            params.root_cpuct()
        } else {
            params.cpuct()
        };

        // scale CPUCT as visits increase
        let scale = params.cpuct_visits_scale() * 128.0;
        cpuct *= 1.0 + ((node.visits() as f32 + scale) / scale).ln();

        // scale CPUCT with variance of Q
        if node.visits() > 1 {
            let mut frac = node.var().sqrt() / params.cpuct_var_scale();
            frac += (1.0 - frac) / (1.0 + params.cpuct_var_warmup() * node.visits() as f32);
            cpuct *= 1.0 + params.cpuct_var_weight() * (frac - 1.0);
        }

        cpuct
    }

    /// Base Exploration Scaling
    ///
    /// Larger value implies more exploration.
    fn base_explore_scaling(params: &MctsParams, node: &Node) -> f32 {
        (params.expl_tau() * (node.visits().max(1) as f32).ln()).exp()
    }

    /// Exploration Scaling
    ///
    /// Larger value implies more exploration.
    pub fn get_explore_scaling(params: &MctsParams, node: &Node) -> f32 {
        #[cfg(not(feature = "datagen"))]
        {
            let mut scale = Self::base_explore_scaling(params, node);

            let gini = node.gini_impurity();
            scale *= (params.gini_base() - params.gini_ln_multiplier() * (gini + 0.001).ln())
                .min(params.gini_min());
            scale
        }

        #[cfg(feature = "datagen")]
        Self::base_explore_scaling(params, node)
    }

    /// Common depth PST
    pub fn get_pst(depth: usize, q: f32, params: &MctsParams) -> f32 {
        let scalar = q - q.min(params.winning_pst_threshold());
        let t = scalar / (1.0 - params.winning_pst_threshold());
        let base_pst = ((depth as f32) - 0.34).powf(-1.8) + 0.9;
        base_pst + (params.winning_pst_max() - base_pst) * t
    }

    /// First Play Urgency
    ///
    /// #### Note
    /// Must return a value in [0, 1].
    pub fn get_fpu(node: &Node) -> f32 {
        1.0 - node.q()
    }

    /// Get a predicted win probability for an action
    ///
    /// #### Note
    /// Must return a value in [0, 1].
    pub fn get_action_value(node: &Node, fpu: f32) -> f32 {
        if node.visits() == 0 {
            fpu
        } else {
            node.q()
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

    pub fn soft_time_cutoff(
        searcher: &Searcher,
        timer: &Instant,
        previous_score: f32,
        best_move_changes: i32,
        nodes: usize,
        time: u128,
    ) -> (bool, f32) {
        let elapsed = timer.elapsed().as_millis();

        // Use more time if our eval is falling, and vice versa
        let (_, mut score) = searcher.get_pv(0);
        score = Searcher::get_cp(score);
        let eval_diff = if previous_score == f32::NEG_INFINITY {
            0.0
        } else {
            previous_score - score
        };
        let falling_eval = (1.0 + eval_diff * searcher.params.tm_falling_eval1()).clamp(
            searcher.params.tm_falling_eval2(),
            searcher.params.tm_falling_eval3(),
        );

        // Use more time if our best move is changing frequently
        let best_move_instability = (1.0
            + (best_move_changes as f32 * searcher.params.tm_bmi1()).ln_1p())
        .clamp(searcher.params.tm_bmi2(), searcher.params.tm_bmi3());

        // Use less time if our best move has a large percentage of visits, and vice versa
        let (best_child_ptr, _, _) = searcher.get_best_action(searcher.tree.root_node());
        let nodes_effort = searcher.tree[best_child_ptr].visits() as f32 / nodes as f32;
        let best_move_visits = (searcher.params.tm_bmv1()
            - ((nodes_effort + searcher.params.tm_bmv2()) * searcher.params.tm_bmv3()).ln_1p()
                * searcher.params.tm_bmv4())
        .clamp(searcher.params.tm_bmv5(), searcher.params.tm_bmv6());

        let total_time =
            (time as f32 * falling_eval * best_move_instability * best_move_visits) as u128;

        (elapsed >= total_time, score)
    }
}
