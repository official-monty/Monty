mod params;
mod tree;

pub use params::MctsParams;
pub use tree::{Node, Tree};

use crate::game::{GameRep, GameState};

use std::{fmt::Write, time::Instant};

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
}

pub struct Searcher<T: GameRep> {
    root_position: T,
    tree: Tree<T>,
    selection: Vec<i32>,
    params: MctsParams,
}

impl<T: GameRep> Searcher<T> {
    pub fn new(root_position: T, tree: Tree<T>, params: MctsParams) -> Self {
        Self {
            root_position,
            tree,
            selection: Vec::new(),
            params,
        }
    }

    pub fn tree_and_board(self) -> (Tree<T>, T) {
        (self.tree, self.root_position)
    }

    fn selected(&self) -> i32 {
        *self.selection.last().unwrap()
    }

    fn pick_child(&self, ptr: i32) -> i32 {
        let node = &self.tree[ptr];
        let expl = self.params.cpuct() * (node.visits().max(1) as f32).sqrt();

        let mut best_idx = -1;
        let mut best_uct = f32::NEG_INFINITY;

        let fpu = if node.visits() > 0 {
            1.0 - node.q()
        } else {
            0.5
        };

        self.tree.map_children(ptr, |child_idx, child| {
            let uct = if child.visits() == 0 {
                fpu + expl * child.policy()
            } else {
                let q = child.q();
                let u = expl * child.policy() / (1 + child.visits()) as f32;

                q + u
            };

            if uct > best_uct {
                best_uct = uct;
                best_idx = child_idx;
            }
        });

        best_idx
    }

    fn select_leaf(&mut self, pos: &mut T) {
        self.selection.clear();
        self.selection.push(0);

        let mut node_ptr = 0;

        loop {
            let node = &self.tree[node_ptr];

            if node.is_terminal() {
                break;
            }

            if node.visits() == 1 && !node.has_children() {
                self.tree.expand(node_ptr, pos);
            }

            let next = self.pick_child(node_ptr);

            if next == -1 {
                break;
            }

            let mov = self.tree[next].mov();
            pos.make_move(mov);

            self.selection.push(next);
            node_ptr = next;
        }
    }

    fn expand_and_simulate(&mut self, pos: &T) -> f32 {
        let node_ptr = self.selected();
        let state = self.tree[node_ptr].get_state(pos);

        match state {
            GameState::Ongoing => pos.get_value(),
            GameState::Draw => 0.5,
            GameState::Lost => -self.params.mate_bonus(),
            GameState::Won => 1.0 + self.params.mate_bonus(),
        }
    }

    fn backprop(&mut self, mut result: f32) {
        while let Some(node_ptr) = self.selection.pop() {
            result = 1.0 - result;
            self.tree[node_ptr].update(1, result);
        }
    }

    fn get_pv(&self, mut depth: usize) -> (Vec<T::Move>, f32) {
        let mut idx = self.tree.get_best_child(0);
        let score = self.tree[idx].q();
        let mut pv = Vec::new();

        while depth > 0 && idx != -1 {
            let node = &self.tree[idx];
            let mov = node.mov();
            pv.push(mov);

            idx = self.tree.get_best_child(idx);
            depth -= 1;
        }

        (pv, score)
    }

    fn search_report(&self, depth: usize, timer: &Instant, nodes: usize) {
        let (pv_line, score) = self.get_pv(depth);

        let elapsed = timer.elapsed();
        let nps = nodes as f32 / elapsed.as_secs_f32();
        let ms = elapsed.as_millis();

        let cp = -400.0 * (1.0 / score.clamp(0.0, 1.0) - 1.0).ln();

        let pv = pv_line.iter().fold(String::new(), |mut pv_str, mov| {
            write!(&mut pv_str, "{} ", self.root_position.conv_mov_to_str(*mov)).unwrap();
            pv_str
        });

        println!(
            "info depth {depth} score cp {cp:.0} time {ms} nodes {nodes} nps {nps:.0} pv {pv}"
        );
    }

    pub fn search(
        &mut self,
        limits: Limits,
        uci_output: bool,
        total_nodes: &mut usize,
        prev_board: &Option<T>,
    ) -> (T::Move, f32) {
        let timer = Instant::now();

        // attempt to reuse the current tree
        if !self.tree.is_empty() {
            if let Some(board) = prev_board {
                println!("info string searching for subtree");
                let ptr = self.tree.recurse_find(0, board, &self.root_position, 2);
                if ptr == -1 || !self.tree[ptr].has_children() {
                    self.tree.clear();
                } else {
                    let mut subtree = Tree::default();
                    let root = self.tree.construct_subtree(ptr, &mut subtree);

                    self.tree = subtree;
                    self.tree[root].make_root();

                    println!(
                        "info string found subtree of size {} nodes",
                        self.tree.len()
                    );
                }
            } else {
                self.tree.clear();
            }
        }

        if self.tree.is_empty() {
            self.tree.push(Node::default());
            self.tree.expand(0, &self.root_position);
        }

        let mut nodes = 0;
        let mut depth = 0;
        let mut cumulative_depth = 0;

        while nodes < limits.max_nodes {
            nodes += 1;

            let mut pos = self.root_position.clone();

            self.select_leaf(&mut pos);

            let this_depth = self.selection.len() - 1;
            cumulative_depth += this_depth;
            let avg_depth = cumulative_depth / nodes;

            let result = self.expand_and_simulate(&pos);

            self.backprop(result);

            if let Some(time) = limits.max_time {
                if nodes % 128 == 0 && timer.elapsed().as_millis() >= time {
                    break;
                }
            }

            if avg_depth > depth {
                depth = avg_depth;

                if uci_output {
                    self.search_report(depth, &timer, nodes);
                }

                if depth >= limits.max_depth {
                    break;
                }
            }
        }

        *total_nodes += nodes;

        if uci_output {
            self.search_report(depth, &timer, nodes);
        }

        let best_idx = self.tree.get_best_child(0);
        let best_child = &self.tree[best_idx];
        (best_child.mov(), best_child.q())
    }
}
