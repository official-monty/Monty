use crate::{
    games::{GameRep, GameState},
    params::MctsParams,
    tree::{Node, Tree},
};

use std::time::Instant;

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
}

pub struct Searcher<'a, T: GameRep> {
    root_position: T,
    tree: Tree,
    params: MctsParams,
    policy: &'a T::Policy,
    value: &'a T::Value,
    datagen: bool,
}

impl<'a, T: GameRep> Searcher<'a, T> {
    pub fn new(
        root_position: T,
        tree: Tree,
        params: MctsParams,
        policy: &'a T::Policy,
        value: &'a T::Value,
        datagen: bool,
    ) -> Self {
        Self {
            root_position,
            tree,
            params,
            policy,
            value,
            datagen,
        }
    }

    pub fn search(
        &mut self,
        limits: Limits,
        uci_output: bool,
        total_nodes: &mut usize,
        prev_board: &Option<T>,
    ) -> (T::Move, f32) {
        let timer = Instant::now();

        // attempt to reuse the current tree stored in memory
        self.tree.try_use_subtree(&self.root_position, prev_board);
        let node = self.tree.root_node();

        // relabel root policies with root PST value
        if self.tree[node].has_children() {
            self.tree[node].relabel_policy(&self.root_position, &self.params, self.policy);
        } else {
            self.tree[node].expand::<T, true>(&self.root_position, &self.params, self.policy);
        }

        let mut nodes = 0;
        let mut depth = 0;
        let mut cumulative_depth = 0;

        // search loop
        loop {
            let mut pos = self.root_position.clone();
            let mut this_depth = 0;
            self.perform_one_iteration(&mut pos, self.tree.root_node(), &mut this_depth);

            cumulative_depth += this_depth - 1;

            // proven checkmate
            if self.tree[self.tree.root_node()].is_terminal() {
                break;
            }

            if nodes >= limits.max_nodes {
                break;
            }

            nodes += 1;

            if let Some(time) = limits.max_time {
                if nodes % 128 == 0 && timer.elapsed().as_millis() >= time {
                    break;
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

        *total_nodes += nodes;

        if uci_output {
            self.search_report(depth.max(1), &timer, nodes);
        }

        let best_action = self.tree.get_best_child(self.tree.root_node());
        let best_child = &self.tree.edge(self.tree.root_node(), best_action);
        (T::Move::from(best_child.mov()), best_child.q())
    }

    fn perform_one_iteration(&mut self, pos: &mut T, ptr: i32, depth: &mut usize) -> f32 {
        *depth += 1;

        self.tree.make_recently_used(ptr);

        let hash = self.tree[ptr].hash();
        let parent = self.tree[ptr].parent();
        let action = self.tree[ptr].action();

        let mut child_state = GameState::Ongoing;
        let pvisits = self.tree.edge(parent, action).visits();

        let mut u = if self.tree[ptr].is_terminal() || pvisits == 0 {
            // probe hash table to use in place of network
            if self.tree[ptr].state() == GameState::Ongoing {
                if let Some(entry) = self.tree.probe_hash(hash) {
                    1.0 - entry.wins / entry.visits as f32
                } else {
                    self.get_utility(ptr, pos)
                }
            } else {
                self.get_utility(ptr, pos)
            }
        } else {
            // expand node on the second visit
            if self.tree[ptr].is_not_expanded() {
                self.tree[ptr].expand::<T, false>(pos, &self.params, self.policy);
            }

            // select action to take via PUCT
            let action = self.pick_action(ptr);

            let edge = self.tree.edge(ptr, action);
            pos.make_move(T::Move::from(edge.mov()));

            let mut child_ptr = edge.ptr();

            // create and push node if not present
            if child_ptr == -1 {
                let state = pos.game_state();
                child_ptr = self.tree.push(Node::new(state, pos.hash(), ptr, action));
                self.tree.edge_mut(ptr, action).set_ptr(child_ptr);
            }

            child_state = self.tree[child_ptr].state();
            self.perform_one_iteration(pos, child_ptr, depth)
        };

        // flip perspective of score
        u = 1.0 - u;
        self.tree.edge_mut(parent, action).update(u);

        let edge = self.tree.edge(parent, action);
        self.tree.push_hash(hash, edge.visits(), edge.wins());

        self.tree.propogate_proven_mates(ptr, child_state);

        self.tree.make_recently_used(ptr);

        u
    }

    fn get_utility(&self, ptr: i32, pos: &T) -> f32 {
        match self.tree[ptr].state() {
            GameState::Ongoing => pos.get_value_wdl(self.value),
            GameState::Draw => 0.5,
            GameState::Lost(_) => 0.0,
            GameState::Won(_) => 1.0,
        }
    }

    fn pick_action(&self, ptr: i32) -> usize {
        if !self.tree[ptr].has_children() {
            panic!("trying to pick from no children!");
        }

        let node = &self.tree[ptr];

        let parent = node.parent();
        let action = node.action();
        let edge = self.tree.edge(parent, action);

        // baseline CPUCT value
        let mut cpuct = if self.datagen && ptr == self.tree.root_node() {
            self.params.root_cpuct()
        } else {
            self.params.cpuct()
        };

        // scale CPUCT as visits increase
        cpuct *= 1.0 + (((edge.visits() + 8192) / 8192) as f32).ln();

        // scale CPUCT with variance of Q
        if edge.visits() > 1 {
            let frac = edge.var().sqrt() / self.params.cpuct_var_scale();
            cpuct *= 1.0 + self.params.cpuct_var_weight() * (frac - 1.0);
        }

        // exploration factor to apply
        let expl = cpuct * (edge.visits().max(1) as f32).sqrt();

        // first play urgency
        let fpu = 1.0 - edge.q();

        let mut best = usize::MAX;
        let mut max = f32::NEG_INFINITY;

        for (i, action) in node.actions().iter().enumerate() {
            let puct = if action.visits() == 0 {
                fpu + expl * action.policy()
            } else {
                action.q() + expl * action.policy() / (1 + action.visits()) as f32
            };

            if puct > max {
                max = puct;
                best = i;
            }
        }

        best
    }

    fn search_report(&self, depth: usize, timer: &Instant, nodes: usize) {
        print!("info depth {depth} ");
        let (pv_line, score) = self.get_pv(depth);

        if score > 1.0 {
            print!("score mate {} ", (pv_line.len() + 1) / 2);
        } else if score < 0.0 {
            print!("score mate -{} ", pv_line.len() / 2);
        } else {
            let cp = -400.0 * (1.0 / score.clamp(0.0, 1.0) - 1.0).ln();
            print!("score cp {cp:.0} ");
        }

        let elapsed = timer.elapsed();
        let nps = nodes as f32 / elapsed.as_secs_f32();
        let ms = elapsed.as_millis();
        let hf = self.tree.len() * 1000 / self.tree.cap();

        print!("time {ms} nodes {nodes} nps {nps:.0} hashfull {hf} pv");

        for mov in pv_line {
            print!(" {}", self.root_position.conv_mov_to_str(mov));
        }

        println!();
    }

    fn get_pv(&self, mut depth: usize) -> (Vec<T::Move>, f32) {
        let mate = self.tree[self.tree.root_node()].is_terminal();

        let idx = self.tree.get_best_child(self.tree.root_node());
        let mut action = self.tree.edge(self.tree.root_node(), idx);

        let score = if action.ptr() != -1 {
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

        while (mate || depth > 0) && action.ptr() != -1 {
            pv.push(T::Move::from(action.mov()));
            let idx = self.tree.get_best_child(action.ptr());

            if idx == usize::MAX {
                break;
            }

            action = self.tree.edge(action.ptr(), idx);
            depth -= 1;
        }

        (pv, score)
    }

    pub fn tree_and_board(self) -> (Tree, T) {
        (self.tree, self.root_position)
    }

    pub fn display_moves(&self) {
        for action in self.tree[self.tree.root_node()].actions() {
            let mov = self.root_position.conv_mov_to_str(action.mov().into());
            let q = action.q() * 100.0;
            println!("{mov} -> {q:.2}%");
        }
    }
}
