mod params;
mod tree;

pub use params::MctsParams;
pub use tree::{Mark, Node, Tree};

use crate::game::{GameRep, GameState};

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
    selection: Vec<i32>,
    params: MctsParams,
}

impl<T: GameRep> Searcher<T> {
    pub fn new(root_position: T, tree: Tree, params: MctsParams) -> Self {
        Self {
            root_position,
            tree,
            selection: Vec::new(),
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
            let node = self.tree.push(Node::new(Mark::Var1));
            self.tree.make_root_node(node);
            self.tree
                .expand::<T, true>(node, &self.root_position, &self.params);
        } else {
            self.tree
                .relabel_policy(self.tree.root_node(), &self.root_position, &self.params);
        }

        let mut nodes = 0;
        let mut depth = 0;
        let mut cumulative_depth = 0;

        // search until a further iteration may overflow the tree
        while self.tree.remaining() > T::MAX_MOVES {
            nodes += 1;

            // start from the root
            let mut pos = self.root_position.clone();

            // step 1: select a leaf node to expand,
            // where a leaf node is defined as a node
            // which has children that have not yet been
            // expanded
            self.select_leaf(&mut pos);

            let this_depth = self.selection.len() - 1;
            cumulative_depth += this_depth;
            let avg_depth = cumulative_depth / nodes;

            // step 2 + 3: expand the node (mostly delayed to
            // second visit) and simulate the game outcome
            let result = self.expand_and_simulate(&pos);

            // step 4: backpropogate the result to the root
            self.backprop(result);

            if self.tree[self.tree.root_node()].is_terminal() {
                break;
            }

            // check if hit node limit
            if nodes >= limits.max_nodes {
                break;
            }

            // check for timeup
            if let Some(time) = limits.max_time {
                if nodes % 128 == 0 && timer.elapsed().as_millis() >= time {
                    break;
                }
            }

            // we define "depth" in the UCI sense as the average
            // depth of selection
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
            self.search_report(depth, &timer, nodes);
        }

        let best_idx = self.tree.get_best_child(self.tree.root_node());
        let best_child = &self.tree[best_idx];
        (T::Move::from(best_child.mov()), best_child.q())
    }

    fn select_leaf(&mut self, pos: &mut T) {
        // always start from the root
        let mut node_ptr = self.tree.root_node();

        self.selection.clear();
        self.selection.push(node_ptr);

        loop {
            let node = &self.tree[node_ptr];

            // if the node is terminal we can go no further,
            // we simply backpropogate the terminal score
            if node.is_terminal() {
                break;
            }

            // this is "expanding on the second visit",
            // an important optimisation - not only does it
            // massively reduce memory usage, it also is a
            // large speedup (avoids many policy net calculations)
            if node.visits() == 1 && !node.has_children() {
                self.tree.expand::<T, false>(node_ptr, pos, &self.params);
            }

            // pick the next child based on PUCT score
            let next = self.pick_child(node_ptr);

            // if there were no children, this node is a leaf
            // node, and so we select it for expansion
            if next == -1 {
                break;
            }

            // descend down the tree
            let mov = self.tree[next].mov();
            pos.make_move(T::Move::from(mov));
            self.selection.push(next);
            node_ptr = next;
        }
    }

    fn expand_and_simulate(&mut self, pos: &T) -> f32 {
        // selected node
        let node_ptr = self.selected();

        // partial expansion (adding children to tree
        // is delayed to the second visit)
        // game state stored relative to nstm from
        // perspective of this node
        let mut state = self.tree[node_ptr].state();
        if state == GameState::Ongoing {
            state = pos.game_state();
            self.tree[node_ptr].set_state(state);
        }

        // simulate the game outcome
        match state {
            GameState::Ongoing => pos.get_value_wdl(),
            GameState::Draw => 0.5,
            GameState::Lost => 0.0,
            GameState::Won => 1.0,
        }
    }

    fn backprop(&mut self, mut result: f32) {
        let mut prev = GameState::Ongoing;
        while let Some(node_ptr) = self.selection.pop() {
            // flip result
            result = 1.0 - result;

            // for a `node` with given stm, `node.wins`
            // is stored from the nstm perspective, for
            // simplicity when it is used
            self.tree[node_ptr].update(1, result);

            if prev == GameState::Lost {
                self.tree[node_ptr].set_state(GameState::Won);
            }

            prev = self.tree[node_ptr].state();
        }
    }

    fn pick_child(&mut self, ptr: i32) -> i32 {
        if !self.tree[ptr].has_children() {
            return -1;
        }

        let is_root = ptr == self.tree.root_node();
        let cpuct = if is_root {
            self.params.root_cpuct()
        } else {
            self.params.cpuct()
        };
        let node = &self.tree[ptr];

        // exploration factor to apply
        let expl = cpuct * (node.visits().max(1) as f32).sqrt();

        // first play urgency - choose a Q value for
        // moves which have no been played yet
        let fpu = if node.visits() > 0 {
            1.0 - node.q()
        } else {
            0.5
        };

        let mut proven_loss = true;

        // return child with highest PUCT score
        let best = self.tree.get_best_child_by_key(ptr, |child| {
            if child.state() != GameState::Won {
                proven_loss = false;
            }

            let q = if child.visits() == 0 { fpu } else { child.q() };

            let u = expl * child.policy() / (1 + child.visits()) as f32;

            q + u
        });

        if proven_loss {
            self.tree[ptr].set_state(GameState::Lost);
            return -1;
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
        let mut idx = self.tree.get_best_child(self.tree.root_node());
        let score = self.tree[idx].q();
        let mut pv = Vec::new();

        while depth > 0 && idx != -1 {
            let node = &self.tree[idx];
            let mov = node.mov();
            pv.push(T::Move::from(mov));

            idx = self.tree.get_best_child(idx);
            depth -= 1;
        }

        (pv, score)
    }

    pub fn tree_and_board(self) -> (Tree, T) {
        (self.tree, self.root_position)
    }

    fn selected(&self) -> i32 {
        *self.selection.last().unwrap()
    }

    pub fn display_moves(&self) {
        self.tree.map_children(self.tree.root_node(), |_, child| {
            let mov = self.root_position.conv_mov_to_str(child.mov().into());
            let q = child.q() * 100.0;
            println!("{mov} -> {q:.2}%");
        });
    }
}
