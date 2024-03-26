use crate::{params::TunableParams, qsearch::quiesce};

use monty_core::{cp_wdl, Castling, GameState, Move, MoveList, PolicyNetwork, Position};

use std::{fmt::Write, time::Instant};

#[derive(Clone, Default)]
pub struct Node {
    visits: i32,
    wins: f32,
    left: usize,
    state: GameState,
    pub moves: MoveList,
}

impl Node {
    fn new(pos: &Position, stack: &[u64], castling: &Castling) -> Self {
        let moves = pos.gen::<true>(castling);
        let state = pos.game_state(&moves, stack);
        Self {
            state,
            ..Default::default()
        }
    }

    fn expand(&mut self, pos: &Position, params: &PolicyNetwork, castling: &Castling) {
        self.moves = pos.gen::<true>(castling);
        self.moves.set_policies(pos, params);
        self.left = self.moves.len();
    }

    fn is_terminal(&self) -> bool {
        self.state != GameState::Ongoing
    }

    pub fn visits(&self) -> i32 {
        self.visits
    }
}

pub struct Searcher<'a> {
    pub castling: Castling,
    pub startpos: Position,
    pub startstack: Vec<u64>,
    pub tree: Vec<Node>,
    pos: Position,
    stack: Vec<u64>,
    node_limit: usize,
    selection: Vec<i32>,
    params: TunableParams,
    policy: &'a PolicyNetwork,
}

impl<'a> Searcher<'a> {
    pub fn new(
        castling: Castling,
        pos: Position,
        stack: Vec<u64>,
        node_limit: usize,
        params: TunableParams,
        policy: &'a PolicyNetwork,
    ) -> Self {
        Self {
            castling,
            startpos: pos,
            startstack: stack.clone(),
            pos,
            tree: Vec::new(),
            stack,
            node_limit,
            selection: Vec::new(),
            params,
            policy,
        }
    }

    pub fn set(
        &mut self,
        pos: Position,
        stack: Vec<u64>,
        node_limit: usize,
        params: TunableParams,
        policy: &'a PolicyNetwork,
        tree: Vec<Node>,
    ) {
        self.startpos = pos;
        self.startstack = stack.clone();
        self.pos = pos;
        self.tree = tree;
        self.stack = stack;
        self.node_limit = node_limit;
        self.selection = Vec::new();
        self.params = params;
        self.policy = policy;
    }

    fn make_move(&mut self, mov: Move) {
        self.stack.push(self.pos.hash());
        self.pos.make(mov, None, &self.castling);
    }

    fn selected(&self) -> i32 {
        *self.selection.last().unwrap()
    }

    fn pick_child(&self, node: &Node) -> usize {
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

    fn select_leaf(&mut self) {
        self.pos = self.startpos;
        self.stack = self.startstack.clone();
        self.selection.clear();
        self.selection.push(0);

        let mut node_ptr = 0;

        loop {
            let node = &mut self.tree[node_ptr as usize];

            if node_ptr != 0 && node.visits == 1 {
                node.expand(&self.pos, self.policy, &self.castling);
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

            self.make_move(mov);
            self.selection.push(next);
            node_ptr = next;
        }
    }

    fn expand_node(&mut self) {
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
        self.make_move(mov);

        let new_node = Node::new(&self.pos, &self.stack, &self.castling);
        self.tree.push(new_node);

        let new_ptr = self.tree.len() as i32 - 1;
        let node = &mut self.tree[node_ptr as usize];
        let to_explore = &mut node.moves[node.left];
        to_explore.set_ptr(new_ptr);

        self.selection.push(to_explore.ptr());
    }

    fn simulate(&self) -> f32 {
        let node_ptr = self.selected();

        let node = &self.tree[node_ptr as usize];

        match node.state {
            GameState::Lost => -self.params.mate_bonus(),
            GameState::Draw => 0.5,
            GameState::Ongoing => {
                let accs = self.pos.get_accs();
                let qs = quiesce(&self.pos, &self.castling, &accs, -30_000, 30_000);
                cp_wdl(qs)
            }
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

    fn get_bestmove<const REPORT: bool>(&self, root_node: &Node) -> (Move, f32) {
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
                    mov.to_uci(&self.castling),
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

    fn get_pv(&self) -> (Vec<Move>, f32) {
        let mut node = &self.tree[0];

        let (mut mov, score) = self.get_bestmove::<false>(node);

        let mut pv = Vec::new();

        while mov.ptr() != -1 {
            pv.push(mov);
            node = &self.tree[mov.ptr() as usize];

            if node.moves.is_empty() {
                break;
            }

            mov = self.get_bestmove::<false>(node).0;
        }

        (pv, score)
    }

    fn construct_subtree(&self, node_ptr: i32, subtree: &mut Vec<Node>) {
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

    fn find_mov_ptr(&self, start: i32, mov: &Move) -> i32 {
        if start == -1 {
            return -1;
        }

        let node = &self.tree[start as usize];

        for child_mov in node.moves.iter() {
            if child_mov.is_same(mov) {
                return child_mov.ptr();
            }
        }

        -1
    }

    pub fn search(
        &mut self,
        max_time: Option<u128>,
        max_depth: usize,
        report_moves: bool,
        uci_output: bool,
        total_nodes: &mut usize,
        prevs: Option<(Move, Move)>
    ) -> (Move, f32) {
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
            let mut root_node = Node::new(&self.startpos, &[], &self.castling);
            root_node.expand(&self.startpos, self.policy, &self.castling);
            self.tree.push(root_node);
        }

        let mut nodes = 1;
        let mut depth = 0;
        let mut seldepth = 0;
        let mut cumulative_depth = 0;

        while nodes <= self.node_limit {
            self.select_leaf();

            let this_depth = self.selection.len();
            cumulative_depth += this_depth;
            let avg_depth = cumulative_depth / nodes;
            seldepth = seldepth.max(this_depth);

            if !self.tree[self.selected() as usize].is_terminal() {
                self.expand_node();
            }

            let result = self.simulate();

            self.backprop(result);

            if let Some(time) = max_time {
                if nodes % 128 == 0 && timer.elapsed().as_millis() >= time {
                    break;
                }
            }

            if avg_depth > depth {
                depth = avg_depth;

                if uci_output {
                    let (pv_line, score) = self.get_pv();
                    let elapsed = timer.elapsed();
                    let nps = nodes as f32 / elapsed.as_secs_f32();
                    let pv = pv_line.iter().fold(String::new(), |mut pv_str, mov| {
                        write!(&mut pv_str, "{} ", mov.to_uci(&self.castling)).unwrap();
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
            }

            if depth >= max_depth {
                break;
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
