mod helpers;
mod params;

pub use helpers::SearchHelpers;
pub use params::MctsParams;

use crate::{
    chess::Move,
    tree::{Node, NodePtr, Tree},
    ChessState, GameState, PolicyNetwork, ValueNetwork,
};

use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::Instant,
};

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub opt_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub kld_min_gain: Option<f64>,
}

#[derive(Default)]
pub struct SearchStats {
    pub total_nodes: AtomicUsize,
    pub total_iters: AtomicUsize,
    pub main_iters: AtomicUsize,
    pub avg_depth: AtomicUsize,
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
        search_stats: &SearchStats,
        best_move: &mut Move,
        best_move_changes: &mut i32,
        previous_score: &mut f32,
        previous_kld: &mut Vec<i32>,
        #[cfg(not(feature = "uci-minimal"))] uci_output: bool,
    ) {
        if self.playout_until_full_internal(search_stats, true, || {
            self.check_limits(
                limits,
                timer,
                search_stats,
                best_move,
                best_move_changes,
                previous_score,
                previous_kld,
                #[cfg(not(feature = "uci-minimal"))]
                uci_output,
            )
        }) {
            self.abort.store(true, Ordering::Relaxed);
        }
    }

    fn playout_until_full_worker(&self, search_stats: &SearchStats) {
        let _ = self.playout_until_full_internal(search_stats, false, || false);
    }

    fn playout_until_full_internal<F>(
        &self,
        search_stats: &SearchStats,
        main_thread: bool,
        mut stop: F,
    ) -> bool
    where
        F: FnMut() -> bool,
    {
        loop {
            let mut pos = self.root_position.clone();
            let mut this_depth = 0;

            if self
                .perform_one_iteration(&mut pos, self.tree.root_node(), &mut this_depth)
                .is_none()
            {
                return false;
            }

            search_stats.total_iters.fetch_add(1, Ordering::Relaxed);
            search_stats
                .total_nodes
                .fetch_add(this_depth, Ordering::Relaxed);
            if main_thread {
                search_stats.main_iters.fetch_add(1, Ordering::Relaxed);
            }

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
        search_stats: &SearchStats,
        best_move: &mut Move,
        best_move_changes: &mut i32,
        previous_score: &mut f32,
        previous_kld_state: &mut Vec<i32>,
        #[cfg(not(feature = "uci-minimal"))] uci_output: bool,
    ) -> bool {
        let iters = search_stats.main_iters.load(Ordering::Relaxed);

        if search_stats.total_iters.load(Ordering::Relaxed) >= limits.max_nodes {
            return true;
        }

        if let Some(min_gain) = limits.kld_min_gain {
            let node = &self.tree[self.tree.root_node()];
            let child_ptr = node.actions();

            let mut visit_dist = vec![0; node.num_actions()];
            for (action, visits) in visit_dist.iter_mut().enumerate() {
                *visits = self.tree[*child_ptr + action].visits();
            }

            if let Some(kld_gain) = Node::kld_gain(&visit_dist, previous_kld_state) {
                if kld_gain < min_gain {
                    return true;
                }
            }

            *previous_kld_state = visit_dist;

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
        let total_depth = search_stats.total_nodes.load(Ordering::Relaxed)
            - search_stats.total_iters.load(Ordering::Relaxed);
        let new_depth = total_depth / search_stats.total_iters.load(Ordering::Relaxed);
        if new_depth > search_stats.avg_depth.load(Ordering::Relaxed) {
            search_stats.avg_depth.store(new_depth, Ordering::Relaxed);
            if new_depth >= limits.max_depth {
                return true;
            }

            #[cfg(not(feature = "uci-minimal"))]
            if uci_output {
                self.search_report(
                    new_depth,
                    timer,
                    search_stats.total_nodes.load(Ordering::Relaxed),
                );
            }
        }

        false
    }

    pub fn search(
        &self,
        threads: usize,
        limits: Limits,
        uci_output: bool,
        update_nodes: &mut usize,
        #[cfg(feature = "datagen")]
        use_dirichlet_noise: bool,
        #[cfg(feature = "datagen")]
        temp: f32,
    ) -> (Move, f32, usize) {
        let timer = Instant::now();

        let node = self.tree.root_node();

        // the root node is added to an empty tree, **and not counted** towards the
        // total node count, in order for `go nodes 1` to give the expected result
        if self.tree.is_empty() {
            let ptr = self.tree.push_new_node().unwrap();

            assert_eq!(node, ptr);

            self.tree[ptr].clear();
            self.tree
                .expand_node(ptr, &self.root_position, self.params, self.policy, 1);

            let root_eval = self.root_position.get_value_wdl(self.value, self.params);
            self.tree[ptr].update(1.0 - root_eval);
        }
        // relabel preexisting root policies with root PST value
        else if self.tree[node].has_children() {
            self.tree
                .relabel_policy(node, &self.root_position, self.params, self.policy, 1);

            let first_child_ptr = { *self.tree[node].actions() };

            for action in 0..self.tree[node].num_actions() {
                let ptr = first_child_ptr + action;

                if ptr.is_null() || !self.tree[ptr].has_children() {
                    continue;
                }

                let mut position = self.root_position.clone();
                position.make_move(self.tree[ptr].parent_move());
                self.tree
                    .relabel_policy(ptr, &position, self.params, self.policy, 2);
            }
        }

        // add dirichlet noise in datagen
        #[cfg(feature = "datagen")]
        if use_dirichlet_noise {
            self.tree.add_dirichlet_noise_to_node(node, 0.03, 0.25);
        }

        let search_stats = SearchStats::default();


        let mut best_move = Move::NULL;
        let mut best_move_changes = 0;
        let mut previous_score = f32::NEG_INFINITY;
        let mut previous_kld = Vec::new();

        // search loop
        while !self.abort.load(Ordering::Relaxed) {
            thread::scope(|s| {
                s.spawn(|| {
                    self.playout_until_full_main(
                        &limits,
                        &timer,
                        &search_stats,
                        &mut best_move,
                        &mut best_move_changes,
                        &mut previous_score,
                        &mut previous_kld,
                        #[cfg(not(feature = "uci-minimal"))]
                        uci_output,
                    );
                });

                for _ in 0..threads - 1 {
                    s.spawn(|| self.playout_until_full_worker(&search_stats));
                }
            });

            if !self.abort.load(Ordering::Relaxed) {
                self.tree.flip(true, threads);
            }
        }

        *update_nodes += search_stats.total_nodes.load(Ordering::Relaxed);

        if uci_output {
            self.search_report(
                search_stats.avg_depth.load(Ordering::Relaxed).max(1),
                &timer,
                search_stats.total_nodes.load(Ordering::Relaxed),
            );
        }

        let (_, _mov, q) = self.get_best_action(self.tree.root_node());

        #[cfg(not(feature = "datagen"))]
        let selected_mov = _mov;        

        #[cfg(feature = "datagen")]
        let selected_mov = self.tree.get_best_child_temp(self.tree.root_node(), temp);

        (selected_mov, q, search_stats.total_iters.load(Ordering::Relaxed))
    }

    fn perform_one_iteration(
        &self,
        pos: &mut ChessState,
        ptr: NodePtr,
        depth: &mut usize,
    ) -> Option<f32> {
        *depth += 1;

        let hash = pos.hash();
        let node = &self.tree[ptr];

        let mut u = if node.is_terminal() || node.visits() == 0 {
            if node.visits() == 0 {
                node.set_state(pos.game_state());
            }

            // probe hash table to use in place of network
            if node.state() == GameState::Ongoing {
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
            if node.is_not_expanded() {
                self.tree
                    .expand_node(ptr, pos, self.params, self.policy, *depth)?;
            }

            // this node has now been accessed so we need to move its
            // children across if they are in the other tree half
            self.tree.fetch_children(ptr)?;

            // select action to take via PUCT
            let action = self.pick_action(ptr, node);

            let first_child_ptr = { *node.actions() };
            let child_ptr = first_child_ptr + action;

            let mov = self.tree[child_ptr].parent_move();

            pos.make_move(mov);

            self.tree[child_ptr].inc_threads();

            // acquire lock to avoid issues with desynced setting of
            // game state between threads when threads > 1
            let lock = if self.tree[child_ptr].visits() == 0 {
                Some(node.actions_mut())
            } else {
                None
            };

            // descend further
            let maybe_u = self.perform_one_iteration(pos, child_ptr, depth);

            drop(lock);

            self.tree[child_ptr].dec_threads();

            let u = maybe_u?;

            self.tree
                .propogate_proven_mates(ptr, self.tree[child_ptr].state());

            u
        };

        // node scores are stored from the perspective
        // **of the parent**, as they are usually only
        // accessed from the parent's POV
        u = 1.0 - u;

        let new_q = node.update(u);
        self.tree.push_hash(hash, 1.0 - new_q);

        Some(u)
    }

    fn get_utility(&self, ptr: NodePtr, pos: &ChessState) -> f32 {
        match self.tree[ptr].state() {
            GameState::Ongoing => pos.get_value_wdl(self.value, self.params),
            GameState::Draw => 0.5,
            GameState::Lost(_) => 0.0,
            GameState::Won(_) => 1.0,
        }
    }

    fn pick_action(&self, ptr: NodePtr, node: &Node) -> usize {
        let is_root = ptr == self.tree.root_node();

        let cpuct = SearchHelpers::get_cpuct(self.params, node, is_root);
        let fpu = SearchHelpers::get_fpu(node);
        let expl_scale = SearchHelpers::get_explore_scaling(self.params, node);

        let expl = cpuct * expl_scale;

        self.tree.get_best_child_by_key(ptr, |child| {
            let mut q = SearchHelpers::get_action_value(child, fpu);

            // virtual loss
            let threads = f64::from(child.threads());
            if threads > 0.0 {
                let visits = f64::from(child.visits());
                let q2 = f64::from(q) * visits / (visits + threads);
                q = q2 as f32;
            }

            let u = expl * child.policy() / (1 + child.visits()) as f32;

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
            let idx = self.tree.get_best_child(ptr);

            if idx == usize::MAX {
                break;
            }

            (ptr, mov, _) = self.get_best_action(ptr);
            depth = depth.saturating_sub(1);
        }

        (pv, score)
    }

    fn get_best_action(&self, node: NodePtr) -> (NodePtr, Move, f32) {
        let idx = self.tree.get_best_child(node);
        let ptr = *self.tree[node].actions() + idx;
        let child = &self.tree[ptr];
        (ptr, child.parent_move(), child.q())
    }

    fn get_cp(score: f32) -> f32 {
        -400.0 * (1.0 / score.clamp(0.0, 1.0) - 1.0).ln()
    }

    pub fn display_moves(&self) {
        let first_child_ptr = { *self.tree[self.tree.root_node()].actions() };
        for action in 0..self.tree[self.tree.root_node()].num_actions() {
            let child = &self.tree[first_child_ptr + action];
            let mov = self.root_position.conv_mov_to_str(child.parent_move());
            let q = child.q() * 100.0;
            println!(
                "{mov} -> {q:.2}% V({}) S({})",
                child.visits(),
                child.state()
            );
        }
    }
}
