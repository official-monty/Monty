mod helpers;
mod iteration;
mod params;
mod search_stats;

pub use helpers::SearchHelpers;
pub use params::MctsParams;
pub use search_stats::SearchStats;

use crate::{
    chess::{GameState, Move},
    networks::{PolicyNetwork, ValueNetwork},
    tree::{NodePtr, Tree},
};

#[cfg(feature = "datagen")]
use crate::tree::Node;

use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Instant,
};

#[cfg(feature = "datagen")]
pub type SearchRet = (Move, f32, usize);

#[cfg(not(feature = "datagen"))]
pub type SearchRet = (Move, f32);

pub static REPORT_ITERS: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub opt_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
    #[cfg(feature = "datagen")]
    pub kld_min_gain: Option<f64>,
}

pub struct Searcher<'a> {
    tree: &'a Tree,
    params: &'a MctsParams,
    policy: &'a PolicyNetwork,
    value: &'a ValueNetwork,
    abort: &'a AtomicBool,
}

impl<'a> Searcher<'a> {
    pub fn new(
        tree: &'a Tree,
        params: &'a MctsParams,
        policy: &'a PolicyNetwork,
        value: &'a ValueNetwork,
        abort: &'a AtomicBool,
    ) -> Self {
        Self {
            tree,
            params,
            policy,
            value,
            abort,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn playout_until_full_main(
        &self,
        limits: &Limits,
        timer: &Instant,
        #[cfg(not(feature = "uci-minimal"))] timer_last_output: &mut Instant,
        search_stats: &SearchStats,
        best_move: &mut Move,
        best_move_changes: &mut i32,
        previous_score: &mut f32,
        #[cfg(feature = "datagen")] previous_kld: &mut Vec<i32>,
        #[cfg(not(feature = "uci-minimal"))] uci_output: bool,
        thread_id: usize,
    ) {
        if self.playout_until_full_internal(search_stats, true, thread_id, || {
            self.check_limits(
                limits,
                timer,
                #[cfg(not(feature = "uci-minimal"))]
                timer_last_output,
                search_stats,
                best_move,
                best_move_changes,
                previous_score,
                #[cfg(feature = "datagen")]
                previous_kld,
                #[cfg(not(feature = "uci-minimal"))]
                uci_output,
            )
        }) {
            self.abort.store(true, Ordering::Relaxed);
        }
    }

    fn playout_until_full_worker(&self, search_stats: &SearchStats, thread_id: usize) {
        let _ = self.playout_until_full_internal(search_stats, false, thread_id, || false);
    }

    fn playout_until_full_internal<F>(
        &self,
        search_stats: &SearchStats,
        main_thread: bool,
        thread_id: usize,
        mut stop: F,
    ) -> bool
    where
        F: FnMut() -> bool,
    {
        loop {
            let mut pos = self.tree.root_position().clone();
            let mut this_depth = 0;

            if iteration::perform_one(
                self,
                &mut pos,
                self.tree.root_node(),
                &mut this_depth,
                thread_id,
            )
            .is_none()
            {
                return false;
            }

            search_stats.add_iter(thread_id, this_depth, main_thread);

            // proven checkmate
            if self.tree[self.tree.root_node()].is_terminal() {
                return true;
            }

            // stop signal sent
            if self.abort.load(Ordering::Relaxed) {
                return true;
            }

            if stop() {
                return true;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_limits(
        &self,
        limits: &Limits,
        timer: &Instant,
        #[cfg(not(feature = "uci-minimal"))] timer_last_output: &mut Instant,
        search_stats: &SearchStats,
        best_move: &mut Move,
        best_move_changes: &mut i32,
        previous_score: &mut f32,
        #[cfg(feature = "datagen")] previous_kld_state: &mut Vec<i32>,
        #[cfg(not(feature = "uci-minimal"))] uci_output: bool,
    ) -> bool {
        let iters = search_stats.main_iters();

        if search_stats.total_iters() >= limits.max_nodes {
            return true;
        }

        #[cfg(feature = "datagen")]
        {
            if let Some(min_gain) = limits.kld_min_gain {
                let node = &self.tree[self.tree.root_node()];
                let child_ptr = node.actions();

                // Force i32 element type
                let mut visit_dist: Vec<i32> = vec![0; node.num_actions()];

                for (action, visits) in visit_dist.iter_mut().enumerate() {
                    let v = self.tree[child_ptr + action].visits();
                    // Saturate to i32::MAX (works whether visits() is u32 or usize)
                    let v_i32 = (v as i64).min(i32::MAX as i64) as i32;
                    *visits = v_i32;
                }

                if let Some(kld_gain) = Node::kld_gain(&visit_dist, previous_kld_state) {
                    if kld_gain < min_gain {
                        return true;
                    }
                }
                *previous_kld_state = visit_dist;
            }
        }

        if iters % 128 == 0 {
            if let Some(time) = limits.max_time {
                if timer.elapsed().as_millis() >= time {
                    return true;
                }
            }

            let (_, new_best_move, _) = self.get_best_action(self.tree.root_node());
            if new_best_move != *best_move {
                *best_move = new_best_move;
                *best_move_changes += 1;
            }
        }

        if iters % 4096 == 0 {
            if let Some(time) = limits.opt_time {
                let (should_stop, score) = SearchHelpers::soft_time_cutoff(
                    self,
                    timer,
                    *previous_score,
                    *best_move_changes,
                    iters,
                    time,
                );

                if should_stop {
                    return true;
                }

                if iters % 16384 == 0 {
                    *best_move_changes = 0;
                }

                *previous_score = if *previous_score == f32::NEG_INFINITY {
                    score
                } else {
                    (score + 2.0 * *previous_score) / 3.0
                };
            }
        }

        // define "depth" as the average depth of selection
        let total_depth = search_stats.total_nodes() - search_stats.total_iters();
        let new_depth = total_depth / search_stats.total_iters();
        if new_depth > search_stats.avg_depth.load(Ordering::Relaxed) {
            search_stats.avg_depth.store(new_depth, Ordering::Relaxed);
            if new_depth >= limits.max_depth {
                return true;
            }

            #[cfg(not(feature = "uci-minimal"))]
            if uci_output {
                self.search_report(
                    new_depth,
                    search_stats.seldepth(),
                    timer,
                    search_stats.total_nodes(),
                    search_stats.total_iters(),
                );

                *timer_last_output = Instant::now();
            }
        }

        #[cfg(not(feature = "uci-minimal"))]
        if uci_output && iters % 8192 == 0 && timer_last_output.elapsed().as_secs() >= 15 {
            self.search_report(
                search_stats.avg_depth.load(Ordering::Relaxed),
                search_stats.seldepth(),
                timer,
                search_stats.total_nodes(),
                search_stats.total_iters(),
            );

            *timer_last_output = Instant::now();
        }

        false
    }

    pub fn search(
        &self,
        threads: usize,
        limits: Limits,
        uci_output: bool,
        update_nodes: &mut usize,
        #[cfg(feature = "datagen")] use_dirichlet_noise: bool,
        #[cfg(feature = "datagen")] temp: f32,
    ) -> SearchRet {
        let timer = Instant::now();
        #[cfg(not(feature = "uci-minimal"))]
        let mut timer_last_output = Instant::now();

        let pos = self.tree.root_position();
        let node = self.tree.root_node();

        // the root node is added to an empty tree, **and not counted** towards the
        // total node count, in order for `go nodes 1` to give the expected result
        if self.tree.is_empty() {
            let ptr = self.tree.push_new_node().unwrap();

            assert_eq!(node, ptr);

            self.tree[ptr].clear();
            self.tree
                .expand_node(ptr, pos, self.params, self.policy, 1, 0);

            let root_eval = pos.get_value_wdl(self.value, self.params);
            self.tree[ptr].update(1.0 - root_eval);
        }
        // relabel preexisting root policies with root PST value
        else if self.tree[node].has_children() {
            self.tree
                .relabel_policy(node, pos, self.params, self.policy, 1);

            let first_child_ptr = self.tree[node].actions();

            for action in 0..self.tree[node].num_actions() {
                let ptr = first_child_ptr + action;

                if ptr.is_null() || !self.tree[ptr].has_children() {
                    continue;
                }

                let mut child = pos.clone();
                child.make_move(self.tree[ptr].parent_move());
                self.tree
                    .relabel_policy(ptr, &child, self.params, self.policy, 2);
            }
        }

        // add dirichlet noise in datagen
        #[cfg(feature = "datagen")]
        if use_dirichlet_noise {
            let epsilon = 0.03;
            let alpha: f32 = if cfg!(feature = "policy") { 0.05 } else { 0.25 };

            self.tree.add_dirichlet_noise_to_node(node, epsilon, alpha);
        }

        let search_stats = SearchStats::new(threads);
        let stats_ref = &search_stats;

        let mut best_move = Move::NULL;
        let mut best_move_changes = 0;
        let mut previous_score = f32::NEG_INFINITY;
        #[cfg(feature = "datagen")]
        let mut previous_kld = Vec::new();

        // search loop
        while !self.abort.load(Ordering::Relaxed) {
            thread::scope(|s| {
                s.spawn(|| {
                    self.playout_until_full_main(
                        &limits,
                        &timer,
                        #[cfg(not(feature = "uci-minimal"))]
                        &mut timer_last_output,
                        stats_ref,
                        &mut best_move,
                        &mut best_move_changes,
                        &mut previous_score,
                        #[cfg(feature = "datagen")]
                        &mut previous_kld,
                        #[cfg(not(feature = "uci-minimal"))]
                        uci_output,
                        0,
                    );
                });

                for i in 1..threads {
                    s.spawn(move || self.playout_until_full_worker(stats_ref, i));
                }
            });

            if !self.abort.load(Ordering::Relaxed) {
                self.tree.flip(true, threads);
            }
        }

        *update_nodes += search_stats.total_nodes();

        if uci_output {
            self.search_report(
                search_stats.avg_depth.load(Ordering::Relaxed).max(1),
                search_stats.seldepth(),
                &timer,
                search_stats.total_nodes(),
                search_stats.total_iters(),
            );
        }

        let (_, _mov, q) = self.get_best_action(self.tree.root_node());

        #[cfg(not(feature = "datagen"))]
        {
            let selected_mov = _mov;
            (selected_mov, q)
        }

        #[cfg(feature = "datagen")]
        {
            let selected_mov = self.tree.get_best_child_temp(self.tree.root_node(), temp);
            (selected_mov, q, search_stats.total_iters())
        }
    }

    fn search_report(
        &self,
        depth: usize,
        seldepth: usize,
        timer: &Instant,
        nodes: usize,
        iters: usize,
    ) {
        print!("info depth {depth} seldepth {seldepth} ");
        let (pv_line, score) = self.get_pv(depth);

        if score > 1.0 {
            print!("score mate {} ", pv_line.len().div_ceil(2));
        } else if score < 0.0 {
            print!("score mate -{} ", pv_line.len() / 2);
        } else {
            let cp = Searcher::get_cp(score);
            print!("score cp {cp:.0} ");
        }

        let nodes = if REPORT_ITERS.load(Ordering::Relaxed) {
            iters
        } else {
            nodes
        };
        let elapsed = timer.elapsed();
        let nps = nodes as f32 / elapsed.as_secs_f32();
        let ms = elapsed.as_millis();

        print!("time {ms} nodes {nodes} nps {nps:.0} pv");

        for mov in pv_line {
            print!(" {}", self.tree.root_position().conv_mov_to_str(mov));
        }

        println!();
    }

    fn get_pv(&self, mut depth: usize) -> (Vec<Move>, f32) {
        let mate = self.tree[self.tree.root_node()].is_terminal();

        let (mut ptr, mut mov, q) = self.get_best_action(self.tree.root_node());

        let score = if !ptr.is_null() {
            match self.tree[ptr].state() {
                GameState::Lost(_) => 1.1,
                GameState::Won(_) => -0.1,
                GameState::Draw => 0.5,
                GameState::Ongoing => q,
            }
        } else {
            q
        };

        let mut pv = Vec::new();
        let half = self.tree.half() > 0;

        while (mate || depth > 0) && !ptr.is_null() && ptr.half() == half {
            pv.push(mov);
            let idx = self.get_best_child(ptr);

            if idx == usize::MAX {
                break;
            }

            (ptr, mov, _) = self.get_best_action(ptr);
            depth = depth.saturating_sub(1);
        }

        (pv, score)
    }

    fn get_best_action(&self, node: NodePtr) -> (NodePtr, Move, f32) {
        let idx = self.get_best_child(node);
        let ptr = self.tree[node].actions() + idx;
        let child = &self.tree[ptr];
        (ptr, child.parent_move(), child.q())
    }

    fn get_best_child(&self, node: NodePtr) -> usize {
        self.tree.get_best_child_by_key(node, |child| {
            if child.visits() == 0 {
                f32::NEG_INFINITY
            } else {
                match child.state() {
                    GameState::Lost(n) => 1.0 + f32::from(n),
                    GameState::Won(n) => f32::from(n) - 256.0,
                    GameState::Draw => 0.5,
                    GameState::Ongoing => child.q(),
                }
            }
        })
    }

    fn get_cp(score: f32) -> f32 {
        let clamped_score = score.clamp(0.0, 1.0);
        let deviation = (clamped_score - 0.5).abs();
        let sign = (clamped_score - 0.5).signum();
        if deviation > 0.107 {
            (100.0 + 2923.0 * (deviation - 0.107)) * sign
        } else {
            let adjusted_score = 0.5 + (clamped_score - 0.5).powi(3) * 100.0;
            -200.0 * (1.0 / adjusted_score - 1.0).ln()
        }
    }

    pub fn display_moves(&self) {
        let first_child_ptr = self.tree[self.tree.root_node()].actions();
        for action in 0..self.tree[self.tree.root_node()].num_actions() {
            let child = &self.tree[first_child_ptr + action];
            let mov = self
                .tree
                .root_position()
                .conv_mov_to_str(child.parent_move());
            let q = child.q() * 100.0;
            println!(
                "{mov} -> {q:.2}% V({}) S({})",
                child.visits(),
                child.state()
            );
        }
    }
}
