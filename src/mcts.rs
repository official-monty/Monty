mod helpers;
mod params;

pub use helpers::SearchHelpers;
pub use params::MctsParams;

use crate::{
    chess::Move,
    tree::{ActionStats, Edge, NodePtr, Tree},
    ChessState, GameState, PolicyNetwork, ValueNetwork,
};

use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Instant,
};

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub opt_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
}

pub struct Searcher<'a> {
    root_position: ChessState,
    tree: &'a Tree,
    params: &'a MctsParams,
    policy: &'a PolicyNetwork,
    value: &'a ValueNetwork,
    abort: &'a AtomicBool,
}

impl<'a> Searcher<'a> {
    pub fn new(
        root_position: ChessState,
        tree: &'a Tree,
        params: &'a MctsParams,
        policy: &'a PolicyNetwork,
        value: &'a ValueNetwork,
        abort: &'a AtomicBool,
    ) -> Self {
        Self {
            root_position,
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
        nodes: &mut usize,
        depth: &mut usize,
        cumulative_depth: &mut usize,
        best_move: &mut Move,
        best_move_changes: &mut i32,
        previous_score: &mut f32,
        #[cfg(not(feature = "uci-minimal"))] uci_output: bool,
    ) -> bool {
        self.playout_until_full_internal(nodes, cumulative_depth, |n, cd| {
            self.check_limits(
                limits,
                timer,
                n,
                best_move,
                best_move_changes,
                previous_score,
                depth,
                cd,
                #[cfg(not(feature = "uci-minimal"))]
                uci_output,
            )
        })
    }

    fn playout_until_full_worker(&self, nodes: &mut usize, cumulative_depth: &mut usize) {
        let _ = self.playout_until_full_internal(nodes, cumulative_depth, |_, _| false);
    }

    fn playout_until_full_internal<F>(
        &self,
        nodes: &mut usize,
        cumulative_depth: &mut usize,
        mut stop: F,
    ) -> bool
    where
        F: FnMut(usize, usize) -> bool,
    {
        loop {
            let mut pos = self.root_position.clone();
            let mut this_depth = 0;

            if let Some(u) = self.perform_one_iteration(
                &mut pos,
                self.tree.root_node(),
                self.tree.root_stats(),
                &mut this_depth,
            ) {
                self.tree.root_stats().update(u);
            } else {
                return false;
            }

            *cumulative_depth += this_depth - 1;
            *nodes += 1;

            // proven checkmate
            if self.tree[self.tree.root_node()].is_terminal() {
                return true;
            }

            // stop signal sent
            if self.abort.load(Ordering::Relaxed) {
                return true;
            }

            if stop(*nodes, *cumulative_depth) {
                return true;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_limits(
        &self,
        limits: &Limits,
        timer: &Instant,
        nodes: usize,
        best_move: &mut Move,
        best_move_changes: &mut i32,
        previous_score: &mut f32,
        depth: &mut usize,
        cumulative_depth: usize,
        #[cfg(not(feature = "uci-minimal"))] uci_output: bool,
    ) -> bool {
        if nodes >= limits.max_nodes {
            return true;
        }

        if nodes % 128 == 0 {
            if let Some(time) = limits.max_time {
                if timer.elapsed().as_millis() >= time {
                    return true;
                }
            }

            let new_best_move = self.get_best_move();
            if new_best_move != *best_move {
                *best_move = new_best_move;
                *best_move_changes += 1;
            }
        }

        if nodes % 4096 == 0 {
            // Time management
            if let Some(time) = limits.opt_time {
                let (should_stop, score) = SearchHelpers::soft_time_cutoff(
                    self,
                    timer,
                    *previous_score,
                    *best_move_changes,
                    nodes,
                    time,
                );

                if should_stop {
                    return true;
                }

                if nodes % 16384 == 0 {
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
        let avg_depth = cumulative_depth / nodes;
        if avg_depth > *depth {
            *depth = avg_depth;
            if *depth >= limits.max_depth {
                return true;
            }

            #[cfg(not(feature = "uci-minimal"))]
            if uci_output {
                self.search_report(*depth, timer, nodes);
            }
        }

        false
    }

    pub fn search(
        &self,
        threads: usize,
        limits: Limits,
        uci_output: bool,
        total_nodes: &mut usize,
    ) -> (Move, f32) {
        let timer = Instant::now();

        // attempt to reuse the current tree stored in memory
        let node = self.tree.root_node();

        // relabel root policies with root PST value
        if self.tree[node].has_children() {
            self.tree[node].relabel_policy(&self.root_position, self.params, self.policy);
        } else {
            self.tree[node].expand::<true>(&self.root_position, self.params, self.policy);
        }

        let mut nodes = 0;
        let mut depth = 0;
        let mut cumulative_depth = 0;

        let mut best_move = Move::NULL;
        let mut best_move_changes = 0;
        let mut previous_score = f32::NEG_INFINITY;

        // search loop
        while !self.abort.load(Ordering::Relaxed) {
            let abort = thread::scope(|s| {
                let abort = s.spawn(|| {
                    self.playout_until_full_main(
                        &limits,
                        &timer,
                        &mut nodes,
                        &mut depth,
                        &mut cumulative_depth,
                        &mut best_move,
                        &mut best_move_changes,
                        &mut previous_score,
                        #[cfg(not(feature = "uci-minimal"))]
                        uci_output,
                    )
                });

                for _ in 0..threads - 1 {
                    s.spawn(|| self.playout_until_full_worker(&mut 0, &mut 0));
                }

                abort.join().unwrap()
            });

            if abort {
                self.abort.store(true, Ordering::Relaxed);
            } else {
                self.tree.flip(true, threads);
            }
        }

        *total_nodes += nodes;

        if uci_output {
            self.search_report(depth.max(1), &timer, nodes);
        }

        let best_action = self.get_best_action();
        (Move::from(best_action.mov()), best_action.q())
    }

    fn perform_one_iteration(
        &self,
        pos: &mut ChessState,
        ptr: NodePtr,
        node_stats: &ActionStats,
        depth: &mut usize,
    ) -> Option<f32> {
        *depth += 1;

        let hash = pos.hash();

        let mut child_state = GameState::Ongoing;

        let u = if self.tree[ptr].is_terminal() || node_stats.visits() == 0 {
            // probe hash table to use in place of network
            if self.tree[ptr].state() == GameState::Ongoing {
                if let Some(entry) = self.tree.probe_hash(hash) {
                    entry.q()
                } else {
                    self.get_utility(ptr, pos)
                }
            } else {
                self.get_utility(ptr, pos)
            }
        } else {
            // expand node on the second visit
            if self.tree[ptr].is_not_expanded() {
                self.tree[ptr].expand::<false>(pos, self.params, self.policy);
            }

            // select action to take via PUCT
            let action = self.pick_action(ptr, node_stats);

            let edge = self.tree.edge_copy(ptr, action);

            pos.make_move(Move::from(edge.mov()));

            let child_ptr = self.tree.fetch_node(pos, ptr, edge.ptr(), action)?;

            self.tree[child_ptr].inc_threads();

            let maybe_u = self.perform_one_iteration(pos, child_ptr, &edge.stats(), depth);

            self.tree[child_ptr].dec_threads();

            let u = maybe_u?;

            let new_q = self.tree.update_edge_stats(ptr, action, u);
            self.tree.push_hash(hash, new_q);

            child_state = self.tree[child_ptr].state();

            u
        };

        self.tree.propogate_proven_mates(ptr, child_state);

        Some(1.0 - u)
    }

    fn get_utility(&self, ptr: NodePtr, pos: &ChessState) -> f32 {
        match self.tree[ptr].state() {
            GameState::Ongoing => pos.get_value_wdl(self.value, self.params),
            GameState::Draw => 0.5,
            GameState::Lost(_) => 0.0,
            GameState::Won(_) => 1.0,
        }
    }

    fn pick_action(&self, ptr: NodePtr, node_stats: &ActionStats) -> usize {
        if !self.tree[ptr].has_children() {
            panic!("trying to pick from no children!");
        }

        let is_root = ptr == self.tree.root_node();

        let cpuct = SearchHelpers::get_cpuct(self.params, node_stats, is_root);
        let fpu = SearchHelpers::get_fpu(node_stats);
        let expl_scale = SearchHelpers::get_explore_scaling(self.params, node_stats);

        let expl = cpuct * expl_scale;

        self.tree.get_best_child_by_key(ptr, |action| {
            let q = if !action.ptr().is_null() && self.tree[action.ptr()].threads() > 0 {
                0.0
            } else {
                SearchHelpers::get_action_value(action, fpu)
            };

            let u = expl * action.policy() / (1 + action.visits()) as f32;

            q + u
        })
    }

    fn search_report(&self, depth: usize, timer: &Instant, nodes: usize) {
        print!("info depth {depth} ");
        let (pv_line, score) = self.get_pv(depth);

        if score > 1.0 {
            print!("score mate {} ", (pv_line.len() + 1) / 2);
        } else if score < 0.0 {
            print!("score mate -{} ", pv_line.len() / 2);
        } else {
            let cp = Searcher::get_cp(score);
            print!("score cp {cp:.0} ");
        }

        let elapsed = timer.elapsed();
        let nps = nodes as f32 / elapsed.as_secs_f32();
        let ms = elapsed.as_millis();

        print!("time {ms} nodes {nodes} nps {nps:.0} pv");

        for mov in pv_line {
            print!(" {}", self.root_position.conv_mov_to_str(mov));
        }

        println!();
    }

    fn get_pv(&self, mut depth: usize) -> (Vec<Move>, f32) {
        let mate = self.tree[self.tree.root_node()].is_terminal();

        let mut action = self.get_best_action();

        let score = if !action.ptr().is_null() {
            match self.tree[action.ptr()].state() {
                GameState::Lost(_) => 1.1,
                GameState::Won(_) => -0.1,
                GameState::Draw => 0.5,
                GameState::Ongoing => action.q(),
            }
        } else {
            action.q()
        };

        let mut pv = Vec::new();
        let half = self.tree.half() > 0;

        while (mate || depth > 0) && !action.ptr().is_null() && action.ptr().half() == half {
            pv.push(Move::from(action.mov()));
            let idx = self.tree.get_best_child(action.ptr());

            if idx == usize::MAX {
                break;
            }

            action = self.tree.edge_copy(action.ptr(), idx);
            depth = depth.saturating_sub(1);
        }

        (pv, score)
    }

    fn get_best_action(&self) -> Edge {
        let idx = self.tree.get_best_child(self.tree.root_node());
        self.tree.edge_copy(self.tree.root_node(), idx)
    }

    fn get_best_move(&self) -> Move {
        Move::from(self.get_best_action().mov())
    }

    fn get_cp(score: f32) -> f32 {
        -400.0 * (1.0 / score.clamp(0.0, 1.0) - 1.0).ln()
    }

    pub fn display_moves(&self) {
        for action in self.tree[self.tree.root_node()].actions().iter() {
            let mov = self.root_position.conv_mov_to_str(action.mov().into());
            let q = action.q() * 100.0;
            println!("{mov} -> {q:.2}%");
        }
    }
}
