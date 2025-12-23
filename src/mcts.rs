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
    tree::{Node, NodePtr, Tree},
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

fn calibrate_wdl(win: f32, draw: f32, loss: f32) -> [f32; 3] {
    const W: [[f64; 3]; 3] = [
        [3.75992276, 0.23714723, -1.85080033],
        [-1.87382233, -0.17493249, -1.85294861],
        [-1.88610042, -0.06221474, 3.70374894],
    ];
    const B: [f64; 3] = [2.34454785, -4.07057366, 1.72602581];

    let eps = 1e-12f64;
    let mut pw = f64::from(win).max(eps);
    let mut pd = f64::from(draw).max(eps);
    let mut pl = f64::from(loss).max(eps);

    let z = pw + pd + pl;
    pw /= z;
    pd /= z;
    pl /= z;

    let x0 = pw.ln();
    let x1 = pd.ln();
    let x2 = pl.ln();

    let s0 = W[0][0] * x0 + W[0][1] * x1 + W[0][2] * x2 + B[0];
    let s1 = W[1][0] * x0 + W[1][1] * x1 + W[1][2] * x2 + B[1];
    let s2 = W[2][0] * x0 + W[2][1] * x1 + W[2][2] * x2 + B[2];

    let m = s0.max(s1).max(s2);
    let e0 = (s0 - m).exp();
    let e1 = (s1 - m).exp();
    let e2 = (s2 - m).exp();
    let sum = e0 + e1 + e2;

    [(e0 / sum) as f32, (e1 / sum) as f32, (e2 / sum) as f32]
}

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
        #[cfg(not(feature = "uci-minimal"))] multipv: usize,
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
                #[cfg(not(feature = "uci-minimal"))]
                multipv,
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
        #[cfg(not(feature = "uci-minimal"))] multipv: usize,
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
                    // Saturate to i32::MAX (works whether visits() is u64, or usize)
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

        if iters.is_multiple_of(128) {
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

        if iters.is_multiple_of(4096) {
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

                if iters.is_multiple_of(16384) {
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
                    multipv,
                );

                *timer_last_output = Instant::now();
            }
        }

        #[cfg(not(feature = "uci-minimal"))]
        if uci_output && iters.is_multiple_of(8192) && timer_last_output.elapsed().as_secs() >= 1 {
            self.search_report(
                search_stats.avg_depth.load(Ordering::Relaxed),
                search_stats.seldepth(),
                timer,
                search_stats.total_nodes(),
                search_stats.total_iters(),
                multipv,
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
        multipv: usize,
        update_nodes: &mut usize,
        #[cfg(feature = "datagen")] use_dirichlet_noise: bool,
        #[cfg(feature = "datagen")] temp: f32,
    ) -> SearchRet {
        let timer = Instant::now();
        #[cfg(not(feature = "uci-minimal"))]
        let mut timer_last_output = Instant::now();

        let pos = self.tree.root_position();
        let root_stm = pos.stm();
        let node = self.tree.root_node();

        // the root node is added to an empty tree, **and not counted** towards the
        // total node count, in order for `go nodes 1` to give the expected result
        if self.tree.is_empty() {
            let ptr = self.tree.push_new_node().unwrap();

            assert_eq!(node, ptr);

            self.tree[ptr].clear();
            self.tree
                .expand_node(ptr, pos, self.params, self.policy, 1, 0);

            let eval = pos.eval_with_contempt(self.value, self.params, root_stm);
            let root_score = eval.contempt.score();
            self.tree
                .update_node_stats(ptr, 1.0 - root_score, eval.contempt.draw, 0);
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
            let alpha = 0.03;
            let epsilon: f32 = if cfg!(feature = "policy") { 0.05 } else { 0.25 };

            self.tree.add_dirichlet_noise_to_node(node, alpha, epsilon);
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
                        #[cfg(not(feature = "uci-minimal"))]
                        multipv,
                        0,
                    );
                });

                for i in 1..threads {
                    s.spawn(move || self.playout_until_full_worker(stats_ref, i));
                }
            });

            if !self.abort.load(Ordering::Relaxed) {
                self.tree.flip(true);
            }
        }

        self.tree.flush_root_accumulator();

        *update_nodes += search_stats.total_nodes();

        if uci_output {
            self.search_report(
                search_stats.avg_depth.load(Ordering::Relaxed).max(1),
                search_stats.seldepth(),
                &timer,
                search_stats.total_nodes(),
                search_stats.total_iters(),
                multipv,
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
        multipv: usize,
    ) {
        let elapsed = timer.elapsed();
        let pv_lines = self.multipv_lines(depth, seldepth, nodes, multipv);

        let elapsed_secs = elapsed.as_secs_f32();
        let ms = elapsed.as_millis();

        for (idx, pv_line) in pv_lines.iter().enumerate() {
            let line_depth = if multipv > 1 {
                pv_line.depth.max(1)
            } else {
                depth
            };

            let line_seldepth = if multipv > 1 {
                pv_line.seldepth.max(1)
            } else {
                seldepth
            };

            let line_nodes = if multipv > 1 {
                pv_line.nodes
            } else if REPORT_ITERS.load(Ordering::Relaxed) {
                iters
            } else {
                nodes
            };

            let nps = line_nodes as f32 / elapsed_secs;

            print!("info depth {line_depth} seldepth {line_seldepth} ");
            if multipv > 1 {
                print!("multipv {} ", idx + 1);
            }

            if pv_line.score > 1.0 {
                print!("score mate {} ", pv_line.line.len().div_ceil(2));
            } else if pv_line.score < 0.0 {
                print!("score mate -{} ", pv_line.line.len() / 2);
            } else {
                let (mut scaled, mut cal) = if multipv > 1 {
                    self.get_display_score_for(pv_line.node)
                } else {
                    self.get_display_score()
                };

                if multipv > 1 && pv_line.node != self.tree.root_node() {
                    scaled = -scaled;
                    cal = [cal[2], cal[1], cal[0]];
                }

                let wdl_i = cal.map(|v| (v * 1000.0).round() as i32);
                print!(
                    "score cp {scaled:.0} wdl {} {} {} ",
                    wdl_i[0], wdl_i[1], wdl_i[2]
                )
            }

            let policy = (pv_line.policy * 10000.0).round();

            print!("time {ms} nodes {line_nodes} nps {nps:.0} policy {policy:.0} pv");

            for mov in &pv_line.line {
                print!(" {}", self.tree.root_position().conv_mov_to_str(*mov));
            }

            println!();
        }
    }

    fn get_display_score(&self) -> (f32, [f32; 3]) {
        self.get_display_score_for(self.tree.root_node())
    }

    fn get_display_score_for(&self, node: NodePtr) -> (f32, [f32; 3]) {
        let node_ref = if node.is_null() {
            &self.tree[self.tree.root_node()]
        } else {
            &self.tree[node]
        };

        let draw = node_ref.draw().clamp(0.0, 1.0);

        let score = (1.0 - node_ref.q()).clamp(0.0, 1.0);
        let win = (score - 0.5 * draw).clamp(0.0, 1.0);
        let loss = (1.0 - win - draw).clamp(0.0, 1.0);

        let cal = calibrate_wdl(win, draw, loss);
        let expected = cal[0] + 0.5 * cal[1];

        let s = expected - 0.5;
        let t = s.abs();
        let scaled = (if t <= 0.25 {
            s.signum() * 4.0 * t
        } else {
            s.signum() * 0.25 / (0.5 - t)
        } * 100.0)
            .clamp(-5000.0, 5000.0);

        (scaled, cal)
    }

    fn multipv_lines(
        &self,
        depth: usize,
        seldepth: usize,
        nodes: usize,
        multipv: usize,
    ) -> Vec<PvLine> {
        let children = self.root_children_by_score(multipv.max(1));

        if children.is_empty() {
            return vec![PvLine {
                line: Vec::new(),
                score: 0.0,
                policy: 0.0,
                node: self.tree.root_node(),
                depth,
                seldepth,
                nodes,
            }];
        }

        let mut lines: Vec<PvLine> = children
            .into_iter()
            .map(|(ptr, mov)| self.build_pv_line(ptr, mov, depth))
            .collect();

        if multipv == 1 {
            for line in &mut lines {
                line.depth = depth;
                line.seldepth = seldepth;
                line.nodes = nodes;
            }
        }

        lines
    }

    fn root_children_by_score(&self, limit: usize) -> Vec<(NodePtr, Move)> {
        let root = self.tree.root_node();
        let first_child_ptr = self.tree[root].actions();

        let mut children: Vec<(NodePtr, Move)> = (0..self.tree[root].num_actions())
            .map(|action| {
                let ptr = first_child_ptr + action;
                (ptr, self.tree[ptr].parent_move())
            })
            .collect();

        children.sort_by(|(a_ptr, _), (b_ptr, _)| {
            let a_score = Self::node_order_score(&self.tree[*a_ptr]);
            let b_score = Self::node_order_score(&self.tree[*b_ptr]);

            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        children.truncate(limit.min(children.len()));

        children
    }

    fn node_order_score(node: &Node) -> f32 {
        if node.visits() == 0 {
            return f32::NEG_INFINITY;
        }

        match node.state() {
            GameState::Lost(n) => 1.0 + f32::from(n),
            GameState::Won(n) => f32::from(n) - 256.0,
            GameState::Draw => 0.5,
            GameState::Ongoing => node.q(),
        }
    }

    fn build_pv_line(
        &self,
        start_ptr: NodePtr,
        start_move: Move,
        mut depth: usize,
    ) -> PvLine {
        let mate = self.tree[self.tree.root_node()].is_terminal();
        let policy = if start_ptr.is_null() {
            0.0
        } else {
            self.tree[start_ptr].policy()
        };
        let mut pv = Vec::new();
        let mut ptr = start_ptr;
        let mut mov = start_move;
        let score = if start_ptr.is_null() {
            0.0
        } else {
            self.pv_score(start_ptr, self.tree[start_ptr].q())
        };

        let mut pv_depth = 0;
        let mut pv_seldepth = 0;

        while (mate || depth > 0) && !ptr.is_null() {
            pv.push(mov);
            pv_depth += 1;
            pv_seldepth = pv_seldepth.max(pv_depth);
            let idx = self.get_best_child(ptr);

            if idx == usize::MAX {
                break;
            }

            let (next_ptr, next_mov, _) = self.get_best_action(ptr);
            ptr = next_ptr;
            mov = next_mov;
            depth = depth.saturating_sub(1);
        }

        PvLine {
            line: pv,
            score,
            policy,
            node: start_ptr,
            depth: pv_depth,
            seldepth: pv_seldepth,
            nodes: if start_ptr.is_null() {
                0
            } else {
                self.tree[start_ptr].visits() as usize
            },
        }
    }

    fn pv_score(&self, ptr: NodePtr, q: f32) -> f32 {
        if ptr.is_null() {
            return q;
        }

        match self.tree[ptr].state() {
            GameState::Lost(_) => 1.1,
            GameState::Won(_) => -0.1,
            GameState::Draw => 0.5,
            GameState::Ongoing => q,
        }
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

struct PvLine {
    line: Vec<Move>,
    score: f32,
    policy: f32,
    node: NodePtr,
    depth: usize,
    seldepth: usize,
    nodes: usize,
}
