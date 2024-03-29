use crate::{
    game::{GameRep, GameState},
    moves::{MoveList, MoveType},
    params::TunableParams,
};

use std::{fmt::Write, time::Instant};

#[derive(Clone, Copy)]
pub struct Limits {
    pub max_time: Option<u128>,
    pub max_depth: usize,
    pub max_nodes: usize,
}

#[derive(Clone, Default)]
pub struct Node<T: GameRep> {
    visits: i32,
    wins: f32,
    left: usize,
    state: GameState,
    moves: MoveList<T::Move>,
}

impl<T: GameRep> Node<T> {
    fn new(pos: &T) -> Self {
        let state = pos.game_state();
        Self {
            state,
            ..Default::default()
        }
    }

    fn expand(&mut self, pos: &T, policy: &T::Policy) {
        self.moves = pos.gen_legal_moves();
        pos.set_policies(policy, &mut self.moves);
        self.left = self.moves.len();
    }

    fn is_terminal(&self) -> bool {
        self.state != GameState::Ongoing
    }

    pub fn moves(&self) -> MoveList<T::Move> {
        self.moves.clone()
    }

    pub fn visits(&self) -> i32 {
        self.visits
    }
}

pub struct Searcher<'a, T: GameRep> {
    root_position: T,
    tree: Vec<Node<T>>,
    selection: Vec<i32>,
    policy: &'a T::Policy,
    value: &'a T::Value,
    params: TunableParams,
}

impl<'a, T: GameRep> Searcher<'a, T> {
    pub fn new(
        root_position: T,
        tree: Vec<Node<T>>,
        policy: &'a T::Policy,
        value: &'a T::Value,
        params: TunableParams,
    ) -> Self {
        Self {
            root_position,
            tree,
            selection: Vec::new(),
            policy,
            value,
            params,
        }
    }

    pub fn tree(self) -> Vec<Node<T>> {
        self.tree
    }

    fn selected(&self) -> i32 {
        *self.selection.last().unwrap()
    }

    fn pick_child(&self, node: &Node<T>) -> usize {
        let expl = self.params.cpuct() * (node.visits.max(1) as f32).sqrt();

        let mut best_idx = 0;
        let mut best_uct = 0.0;

        let fpu = if node.visits > 0 {
            1.0 - node.wins / node.visits as f32
        } else {
            0.5
        };

        for (idx, mov) in node.moves.iter().enumerate() {
            let uct = if mov.ptr() == -1 {
                fpu + expl * mov.policy()
            } else {
                let child = &self.tree[mov.ptr() as usize];

                let q = child.wins / child.visits as f32;
                let u = expl * mov.policy() / (1 + child.visits) as f32;

                q + u
            };

            if uct > best_uct {
                best_uct = uct;
                best_idx = idx;
            }
        }

        best_idx
    }

    fn select_leaf(&mut self, pos: &mut T) {
        self.selection.clear();
        self.selection.push(0);

        let mut node_ptr = 0;

        loop {
            let node = &mut self.tree[node_ptr as usize];

            if node_ptr != 0 && node.visits == 1 {
                node.expand(pos, self.policy);
            }

            let node = &self.tree[node_ptr as usize];

            if node.is_terminal() {
                break;
            }

            if node.moves.is_empty() {
                println!("visits {} ptr {}", node.visits, node_ptr);
            }

            let mov_idx = self.pick_child(node);
            let mov = node.moves[mov_idx];
            let next = mov.ptr();

            if next == -1 {
                break;
            }

            pos.make_move(mov);
            self.selection.push(next);
            node_ptr = next;
        }
    }

    fn expand_node(&mut self, pos: &mut T) {
        let node_ptr = self.selected();
        let node = &self.tree[node_ptr as usize];

        assert!(node.left > 0);

        let new_idx = self.pick_child(node);

        let node = &mut self.tree[node_ptr as usize];
        node.left -= 1;

        if node.left > 0 {
            node.moves.swap(new_idx, node.left);
        }

        let mov = node.moves[node.left];
        pos.make_move(mov);

        let new_node = Node::new(pos);
        self.tree.push(new_node);

        let new_ptr = self.tree.len() as i32 - 1;
        let node = &mut self.tree[node_ptr as usize];
        let to_explore = &mut node.moves[node.left];
        to_explore.set_ptr(new_ptr);

        self.selection.push(to_explore.ptr());
    }

    fn simulate(&self, pos: &T) -> f32 {
        let node_ptr = self.selected();

        let node = &self.tree[node_ptr as usize];

        match node.state {
            GameState::Ongoing => pos.get_value(self.value),
            GameState::Draw => 0.5,
            GameState::Lost => -self.params.mate_bonus(),
            GameState::Won => 1.0 + self.params.mate_bonus(),
        }
    }

    fn backprop(&mut self, mut result: f32) {
        while let Some(node_ptr) = self.selection.pop() {
            let node = &mut self.tree[node_ptr as usize];
            node.visits += 1;
            result = 1.0 - result;
            node.wins += result;
        }
    }

    fn get_bestmove<const REPORT: bool>(&self, root_node: &Node<T>) -> (T::Move, f32) {
        let mut best_move = root_node.moves[0];
        let mut best_score = 0.0;

        for mov in root_node.moves.iter() {
            if mov.ptr() == -1 {
                continue;
            }

            let node = &self.tree[mov.ptr() as usize];
            let score = node.wins / node.visits as f32;

            if REPORT {
                println!(
                    "info move {} score wdl {:.2}% ({:.2} / {})",
                    self.root_position.conv_mov_to_str(*mov),
                    score * 100.0,
                    node.wins,
                    node.visits,
                );
            }

            if score > best_score {
                best_score = score;
                best_move = *mov;
            }
        }

        (best_move, best_score)
    }

    fn get_pv(&self, mut depth: usize) -> (Vec<T::Move>, f32) {
        let mut node = &self.tree[0];

        let (mut mov, score) = self.get_bestmove::<false>(node);

        let mut pv = Vec::new();

        while depth > 0 && mov.ptr() != -1 {
            pv.push(mov);
            node = &self.tree[mov.ptr() as usize];

            if node.moves.is_empty() {
                break;
            }

            mov = self.get_bestmove::<false>(node).0;
            depth -= 1;
        }

        (pv, score)
    }

    fn construct_subtree(&self, node_ptr: i32, subtree: &mut Vec<Node<T>>) {
        if node_ptr == -1 {
            return;
        }

        let node = &self.tree[node_ptr as usize];
        subtree.push(node.clone());

        let idx = subtree.len() - 1;

        for (i, mov) in node.moves.iter().enumerate() {
            let new_ptr = mov.ptr();
            let curr_len = subtree.len();

            if new_ptr != -1 {
                subtree[idx].moves[i].set_ptr(curr_len as i32);
                self.construct_subtree(new_ptr, subtree);
            }
        }
    }

    fn find_mov_ptr(&self, start: i32, mov: &T::Move) -> i32 {
        if start == -1 {
            return -1;
        }

        let node = &self.tree[start as usize];

        for child_mov in node.moves.iter() {
            if child_mov.is_same_action(*mov) {
                return child_mov.ptr();
            }
        }

        -1
    }

    pub fn search(
        &mut self,
        limits: Limits,
        report_moves: bool,
        uci_output: bool,
        total_nodes: &mut usize,
        prevs: Option<(T::Move, T::Move)>,
    ) -> (T::Move, f32) {
        let timer = Instant::now();

        // attempt to reuse the previous tree
        if !self.tree.is_empty() {
            if let Some((prev_prev, prev)) = prevs {
                let prev_prev_ptr = self.find_mov_ptr(0, &prev_prev);
                let prev_ptr = self.find_mov_ptr(prev_prev_ptr, &prev);
                if prev_ptr == -1 || self.tree[prev_ptr as usize].visits == 1 {
                    self.tree.clear();
                } else {
                    let mut subtree = Vec::new();
                    self.construct_subtree(prev_ptr, &mut subtree);
                    self.tree = subtree;
                }
            } else {
                self.tree.clear();
            }
        }

        if self.tree.is_empty() {
            let mut root_node = Node::new(&self.root_position);
            root_node.expand(&self.root_position, self.policy);
            self.tree.push(root_node);
        }

        let mut nodes = 1;
        let mut depth = 0;
        let mut seldepth = 0;
        let mut cumulative_depth = 0;

        while nodes <= limits.max_nodes {
            let mut pos = self.root_position.clone();

            self.select_leaf(&mut pos);

            let this_depth = self.selection.len();
            cumulative_depth += this_depth;
            let avg_depth = cumulative_depth / nodes;
            seldepth = seldepth.max(this_depth);

            if !self.tree[self.selected() as usize].is_terminal() {
                self.expand_node(&mut pos);
            }

            let result = self.simulate(&pos);

            self.backprop(result);

            if let Some(time) = limits.max_time {
                if nodes % 128 == 0 && timer.elapsed().as_millis() >= time {
                    break;
                }
            }

            if avg_depth > depth {
                depth = avg_depth;

                if uci_output {
                    let (pv_line, score) = self.get_pv(depth);
                    let elapsed = timer.elapsed();
                    let nps = nodes as f32 / elapsed.as_secs_f32();
                    let pv = pv_line.iter().fold(String::new(), |mut pv_str, mov| {
                        write!(&mut pv_str, "{} ", self.root_position.conv_mov_to_str(*mov))
                            .unwrap();
                        pv_str
                    });

                    println!(
                        "info depth {depth} \
                        seldepth {seldepth} \
                        score cp {:.0} \
                        time {} \
                        nodes {nodes} \
                        nps {nps:.0} \
                        pv {pv}",
                        -400.0 * (1.0 / score.clamp(0.0, 1.0) - 1.0).ln(),
                        elapsed.as_millis(),
                    );
                }

                if depth >= limits.max_depth {
                    break;
                }
            }

            nodes += 1;
        }

        *total_nodes += nodes;

        if report_moves {
            self.get_bestmove::<true>(&self.tree[0])
        } else {
            self.get_bestmove::<false>(&self.tree[0])
        }
    }
}
