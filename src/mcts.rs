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

    pub fn search(
        &mut self,
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
        loop {
            let mut pos = self.root_position.clone();
            let mut this_depth = 0;

            let u = self.perform_one_iteration(
                &mut pos,
                self.tree.root_node(),
                self.tree.root_stats(),
                &mut this_depth,
            );

            self.tree.root_stats().update(u);

            cumulative_depth += this_depth - 1;

            // proven checkmate
            if self.tree[self.tree.root_node()].is_terminal() {
                break;
            }

            if nodes >= limits.max_nodes {
                break;
            }

            if self.abort.load(Ordering::Relaxed) {
                break;
            }

            nodes += 1;

            if nodes % 256 == 0 {
                if let Some(time) = limits.max_time {
                    if timer.elapsed().as_millis() >= time {
                        break;
                    }
                }

                let new_best_move = self.get_best_move();
                if new_best_move != best_move {
                    best_move = new_best_move;
                    best_move_changes += 1;
                }
            }

            if nodes % 16384 == 0 {
                // Time management
                if let Some(time) = limits.opt_time {
                    let elapsed = timer.elapsed().as_millis();

                    // Use more time if our eval is falling, and vice versa
                    let (_, mut score) = self.get_pv(0);
                    score = Searcher::get_cp(score);
                    let eval_diff = if previous_score == f32::NEG_INFINITY {
                        0.0
                    } else {
                        previous_score - score
                    };
                    let falling_eval = (1.0 + eval_diff * 0.05).clamp(0.60, 1.80);

                    // Use more time if our best move is changing frequently
                    let best_move_instability =
                        (1.0 + (best_move_changes as f32 * 0.3).ln_1p()).clamp(1.0, 3.2);

                    // Use less time if our best move has a large percentage of visits, and vice versa
                    let nodes_effort = self.get_best_action().visits() as f32 / nodes as f32;
                    let best_move_visits =
                        (2.5 - ((nodes_effort + 0.3) * 0.55).ln_1p() * 4.0).clamp(0.55, 1.50);

                    let total_time =
                        (time as f32 * falling_eval * best_move_instability * best_move_visits)
                            as u128;
                    if elapsed >= total_time {
                        break;
                    }

                    best_move_changes = 0;
                    previous_score = if previous_score == f32::NEG_INFINITY {
                        score
                    } else {
                        (score + previous_score) / 2.0
                    };
                }
            }

            // define "depth" as the average depth of selection
            let avg_depth = cumulative_depth / nodes;
            if avg_depth > depth {
                depth = avg_depth;
                if depth >= limits.max_depth {
                    break;
                }

                if uci_output {
                    self.search_report(depth, &timer, nodes);
                }
            }
        }

        self.abort.store(true, Ordering::Relaxed);

        *total_nodes += nodes;

        if uci_output {
            self.search_report(depth.max(1), &timer, nodes);
        }

        let best_action = self.tree.get_best_child(self.tree.root_node());
        let best_child = self.tree.edge_copy(self.tree.root_node(), best_action);
        (Move::from(best_child.mov()), best_child.q())
    }

    fn perform_one_iteration(&mut self, pos: &mut ChessState, ptr: NodePtr, node_stats: &ActionStats, depth: &mut usize) -> f32 {
        *depth += 1;

        let hash = pos.hash();

        let mut child_state = GameState::Ongoing;

        let u = if self.tree[ptr].is_terminal() || node_stats.visits() == 0 {
            // probe hash table to use in place of network
            if self.tree[ptr].state() == GameState::Ongoing {
                if let Some(entry) = self.tree.probe_hash(hash) {
                    1.0 - entry.q()
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
            let action = self.pick_action(ptr, &node_stats);

            let edge = self.tree.edge_copy(ptr, action);
            pos.make_move(Move::from(edge.mov()));

            let child_ptr = self.tree.fetch_node(pos, ptr, edge.ptr(), action);

            let u = self.perform_one_iteration(pos, child_ptr, &edge.stats(), depth);

            let new_q = self.tree.update_edge_stats(ptr, action, u);
            self.tree.push_hash(hash, new_q);

            child_state = self.tree[child_ptr].state();

            u
        };

        self.tree.propogate_proven_mates(ptr, child_state);

        1.0 - u
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

        let cpuct = SearchHelpers::get_cpuct(self.params, &node_stats, is_root);
        let fpu = SearchHelpers::get_fpu(&node_stats);
        let expl_scale = SearchHelpers::get_explore_scaling(self.params, &node_stats);

        let expl = cpuct * expl_scale;

        self.tree.get_best_child_by_key(ptr, |action| {
            let q = SearchHelpers::get_action_value(action, fpu);
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

        let idx = self.tree.get_best_child(self.tree.root_node());
        let mut action = self.tree.edge_copy(self.tree.root_node(), idx);

        let score = if action.ptr().is_null() {
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

        while (mate || depth > 0) && action.ptr().is_null() {
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
        let idx = self.tree.get_best_child(self.tree.root_node());
        let action = self.tree.edge_copy(self.tree.root_node(), idx);
        Move::from(action.mov())
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
