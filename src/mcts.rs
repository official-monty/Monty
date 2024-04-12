use crate::{
    games::{GameRep, GameState},
    params::MctsParams,
    tree::{Edge, Node, Tree},
};

use std::time::Instant;

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
}

pub struct Searcher<T: GameRep> {
    root_position: T,
    tree: Tree,
    params: MctsParams,
}

impl<T: GameRep> Searcher<T> {
    pub fn new(root_position: T, tree: Tree, params: MctsParams) -> Self {
        Self {
            root_position,
            tree,
            params,
        }
    }

    /// the main MCTS search function
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

        // we failed to reuse a tree, push the root node to
        // the tree and expand it
        if self.tree.is_empty() {
            let node = self.tree.push(Node::new(GameState::Ongoing, -1, 0));
            self.tree.make_root_node(node);
            self.tree[node].expand::<T, true>(&self.root_position, &self.params);
        } else {
            let node = self.tree.root_node();
            self.tree[node].relabel_policy(&self.root_position, &self.params);
        }

        let mut nodes = 0;
        let mut depth = 0;
        let mut cumulative_depth = 0;

        // search until a further iteration may overflow the tree
        loop {
            // start from the root
            let mut pos = self.root_position.clone();

            let mut this_depth = 0;
            self.perform_one_iteration(&mut pos, self.tree.root_node(), &mut this_depth);

            // update depth statistics
            cumulative_depth += this_depth - 1;

            // proven checkmate
            if self.tree[self.tree.root_node()].is_terminal() {
                break;
            }

            // check if hit node limit
            if nodes >= limits.max_nodes {
                break;
            }

            nodes += 1;

            // check for timeup
            if let Some(time) = limits.max_time {
                if nodes % 128 == 0 && timer.elapsed().as_millis() >= time {
                    break;
                }
            }

            // we define "depth" in the UCI sense as the average
            // depth of selection
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

        let idx = self.tree.get_best_child(self.tree.root_node());

        if idx == usize::MAX {
            self.tree.display::<T>(self.tree.root_node(), 1);
        }

        let best_child = &self.tree.edge(self.tree.root_node(), idx);
        (T::Move::from(best_child.mov()), best_child.q())
    }

    fn perform_one_iteration(&mut self, pos: &mut T, ptr: i32, depth: &mut usize) -> f32 {
        *depth += 1;

        // mark this node as most recently used
        self.tree.make_recently_used(ptr);

        let mut u;
        let child_state;

        let parent = self.tree[ptr].parent();
        let action = self.tree[ptr].action();
        let pvisits = self.tree.edge(parent, action).visits();

        if self.tree[ptr].is_terminal() || pvisits == 0 {
            child_state = GameState::Ongoing;
            u = self.get_utility(ptr, pos);
        } else {
            // this is "expanding on the second visit",
            // an important optimisation - not only does it
            // massively reduce memory usage, it also is a
            // large speedup (avoids many policy net calculations)
            if self.tree[ptr].is_not_expanded() {
                self.tree[ptr].expand::<T, false>(pos, &self.params);
            }

            // select action to take via puct
            let action = self.pick_action(ptr);

            // proved a loss from the child nodes
            if action == usize::MAX {
                child_state = GameState::Ongoing;
                u = self.get_utility(ptr, pos);
            } else {
                let edge = self.tree.edge(ptr, action);
                let mut child_ptr = edge.ptr();

                // descend down the tree
                pos.make_move(T::Move::from(edge.mov()));

                // this node has not yet been pushed to the tree,
                // create it and push it
                if child_ptr == -1 {
                    let state = pos.game_state();
                    child_ptr = self.tree.push(Node::new(state, ptr, action));
                    self.tree.edge_mut(ptr, action).set_ptr(child_ptr);
                }

                child_state = self.tree[child_ptr].state();
                u = self.perform_one_iteration(pos, child_ptr, depth);
            }
        };

        // for convenience the value is stored from the nstm
        // perspective (as it is usually accessed from the
        // parent's perspective)
        u = 1.0 - u;

        self.tree.edge_mut(parent, action).update(u);

        // if the child node resulted in a loss, then
        // this node has a guaranteed win
        if let GameState::Lost(n) = child_state {
            self.tree[ptr].set_state(GameState::Won(n + 1));
        }

        // mark this node as most recently used
        self.tree.make_recently_used(ptr);

        u
    }

    fn get_utility(&self, ptr: i32, pos: &T) -> f32 {
        match self.tree[ptr].state() {
            GameState::Ongoing => pos.get_value_wdl(),
            GameState::Draw => 0.5,
            GameState::Lost(_) => 0.0,
            GameState::Won(_) => 1.0,
        }
    }

    fn pick_action(&mut self, ptr: i32) -> usize {
        if !self.tree[ptr].has_children() {
            panic!("trying to pick from no children!");
        }

        let is_root = ptr == self.tree.root_node();
        let cpuct = if is_root {
            self.params.root_cpuct()
        } else {
            self.params.cpuct()
        };
        let node = &self.tree[ptr];

        let parent = node.parent();
        let action = node.action();
        let edge = self.tree.edge(parent, action);

        // exploration factor to apply
        let expl = cpuct * (edge.visits().max(1) as f32).sqrt();

        // first play urgency - choose a Q value for
        // moves which have no been played yet
        let fpu = 1.0 - edge.q();

        let mut proven_loss = true;
        let mut win_len = 0;
        let mut best = 0;
        let mut max = f32::NEG_INFINITY;

        // return child with highest PUCT score
        for (i, action) in node.actions().iter().enumerate() {
            let puct = if action.visits() == 0 {
                proven_loss = false;
                fpu + expl * action.policy()
            } else {
                if action.ptr() != -1 {
                    let child = &self.tree[action.ptr()];

                    if let GameState::Won(n) = child.state() {
                        win_len = n.max(win_len);
                    } else {
                        proven_loss = false;
                    }
                } else {
                    proven_loss = false;
                }

                action.q() + expl * action.policy() / (1 + action.visits()) as f32
            };

            if puct > max {
                max = puct;
                best = i;
            }
        }

        if proven_loss {
            self.tree[ptr].set_state(GameState::Lost(win_len + 1));
            return usize::MAX;
        }

        best
    }

    fn search_report(&self, depth: usize, timer: &Instant, nodes: usize) {
        print!("info depth {depth} ");
        let (pv_line, score) = self.get_pv(depth);

        if score == 1.0 {
            print!("score mate {} ", (pv_line.len() + 1) / 2);
        } else if score == 0.0 {
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
        let key = |edge: &Edge| {
            if edge.ptr() == -1 {
                -10000.0
            } else {
                let child = &self.tree[edge.ptr()];
                match child.state() {
                    GameState::Draw => 0.5,
                    GameState::Ongoing => edge.q(),
                    GameState::Lost(n) => 1.0 + f32::from(n),
                    GameState::Won(n) => f32::from(n) - 256.0,
                }
            }
        };

        let mate = self.tree[self.tree.root_node()].is_terminal();

        let idx = self.tree.get_best_child_by_key(self.tree.root_node(), key);
        let mut action = self.tree.edge(self.tree.root_node(), idx);

        let score = action.q();
        let mut pv = Vec::new();

        while (mate || depth > 0) && action.ptr() != -1 {
            pv.push(T::Move::from(action.mov()));
            let idx = self.tree.get_best_child_by_key(action.ptr(), key);

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
