mod half;
mod hash;
mod lock;
mod node;

use half::TreeHalf;
use hash::{HashEntry, HashTable};
use node::NodeStatsDelta;
pub use node::{Node, NodePtr};

use std::{
    array,
    mem::MaybeUninit,
    ops::Index,
    sync::atomic::{AtomicBool, AtomicI16, AtomicU64, Ordering},
};

use crate::{
    chess::{ChessState, GameState, Move},
    mcts::{MctsParams, SearchHelpers},
    networks::PolicyNetwork,
};

const NUM_SIDES: usize = 2;
const NUM_SQUARES: usize = 64;
const ROOT_ACCUM_THRESHOLD: u64 = 32;
const ROOT_ACCUM_EAGER_LIMIT: u64 = 256;
const NODE_BATCH_THRESHOLD: u64 = 16384;
const MAX_BATCHED_NODES: usize = 32;
const BATCH_SLOT_RESERVED: u64 = u64::MAX - 1;

#[repr(align(64))]
struct RootAccumulatorEntry {
    visits: AtomicU64,
    sum_q: AtomicU64,
    sum_sq_q: AtomicU64,
}

impl RootAccumulatorEntry {
    fn new() -> Self {
        Self {
            visits: AtomicU64::new(0),
            sum_q: AtomicU64::new(0),
            sum_sq_q: AtomicU64::new(0),
        }
    }

    fn add(&self, delta: NodeStatsDelta) -> Option<NodeStatsDelta> {
        if delta.is_empty() {
            return None;
        }

        let visits_added = delta.visits;
        let previous_visits = self.visits.fetch_add(visits_added, Ordering::AcqRel);
        self.sum_q.fetch_add(delta.sum_q, Ordering::AcqRel);
        self.sum_sq_q.fetch_add(delta.sum_sq_q, Ordering::AcqRel);

        let new_total = previous_visits.saturating_add(visits_added);
        if new_total >= ROOT_ACCUM_THRESHOLD {
            let flush = self.take();
            if flush.is_empty() {
                None
            } else {
                Some(flush)
            }
        } else {
            None
        }
    }

    fn take(&self) -> NodeStatsDelta {
        NodeStatsDelta {
            visits: self.visits.swap(0, Ordering::AcqRel),
            sum_q: self.sum_q.swap(0, Ordering::AcqRel),
            sum_sq_q: self.sum_sq_q.swap(0, Ordering::AcqRel),
        }
    }

    fn reset(&self) {
        self.visits.store(0, Ordering::Relaxed);
        self.sum_q.store(0, Ordering::Relaxed);
        self.sum_sq_q.store(0, Ordering::Relaxed);
    }
}

impl Default for RootAccumulatorEntry {
    fn default() -> Self {
        Self::new()
    }
}

struct RootAccumulator {
    nodes: [AtomicU64; MAX_BATCHED_NODES],
    entries: Vec<[RootAccumulatorEntry; MAX_BATCHED_NODES]>,
}

impl RootAccumulator {
    fn new(threads: usize) -> Self {
        let nodes = array::from_fn(|_| AtomicU64::new(NodePtr::NULL.inner()));
        let mut entries = Vec::with_capacity(threads);
        for _ in 0..threads {
            entries.push(array::from_fn(|_| RootAccumulatorEntry::new()));
        }

        Self { nodes, entries }
    }

    fn add(&self, ptr: NodePtr, node: &Node, delta: NodeStatsDelta, thread_id: usize) {
        if delta.is_empty() {
            return;
        }

        if thread_id >= self.entries.len() {
            node.apply_delta(delta);
            return;
        }

        if ptr.idx() != 0 && node.visits() < NODE_BATCH_THRESHOLD {
            node.apply_delta(delta);
            return;
        }

        let Some(slot) = self.slot_for(ptr) else {
            node.apply_delta(delta);
            return;
        };

        if slot == 0 && node.visits() < ROOT_ACCUM_EAGER_LIMIT {
            node.apply_delta(delta);
            return;
        }

        if let Some(flush) = self.entries[thread_id][slot].add(delta) {
            node.apply_delta(flush);
        }
    }

    fn slot_for(&self, ptr: NodePtr) -> Option<usize> {
        let inner = ptr.inner();

        for (idx, slot) in self.nodes.iter().enumerate() {
            let current = slot.load(Ordering::Acquire);

            if current == inner {
                return Some(idx);
            }

            if current == NodePtr::NULL.inner() && slot
                    .compare_exchange(
                        NodePtr::NULL.inner(),
                        BATCH_SLOT_RESERVED,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_ok() {
                self.clear_slot(idx);
                slot.store(inner, Ordering::Release);
                return Some(idx);
            }
        }

        None
    }

    fn clear_slot(&self, slot: usize) {
        for thread_entries in &self.entries {
            thread_entries[slot].reset();
        }
    }

    fn flush_thread<F>(&self, apply: &mut F, thread_id: usize)
    where
        F: FnMut(NodePtr, NodeStatsDelta),
    {
        if thread_id >= self.entries.len() {
            return;
        }

        for slot in 0..MAX_BATCHED_NODES {
            let pending = self.entries[thread_id][slot].take();

            if pending.is_empty() {
                continue;
            }

            let raw = self.nodes[slot].load(Ordering::Acquire);

            if raw == NodePtr::NULL.inner() || raw == BATCH_SLOT_RESERVED {
                continue;
            }

            apply(NodePtr::from_raw(raw), pending);
        }
    }

    fn flush_all<F>(&self, mut apply: F)
    where
        F: FnMut(NodePtr, NodeStatsDelta),
    {
        for thread_id in 0..self.entries.len() {
            self.flush_thread(&mut apply, thread_id);
        }
    }

    fn reset(&self, root: NodePtr) {
        for (idx, slot) in self.nodes.iter().enumerate() {
            let value = if idx == 0 {
                root.inner()
            } else {
                NodePtr::NULL.inner()
            };
            slot.store(value, Ordering::Relaxed);
        }

        for thread_entries in &self.entries {
            for entry in thread_entries {
                entry.reset();
            }
        }
    }
}

struct ButterflyTable {
    data: Vec<AtomicI16>,
}

impl ButterflyTable {
    fn new() -> Self {
        let capacity = NUM_SIDES * NUM_SQUARES * NUM_SQUARES;
        let mut data = Vec::with_capacity(capacity);
        data.extend((0..capacity).map(|_| AtomicI16::new(0)));
        Self { data }
    }

    fn index(side: usize, from: u16, to: u16) -> usize {
        side * NUM_SQUARES * NUM_SQUARES + usize::from(from) * NUM_SQUARES + usize::from(to)
    }

    fn entry(&self, side: usize, mov: Move) -> &AtomicI16 {
        let idx = Self::index(side, mov.src(), mov.to());
        &self.data[idx]
    }

    fn policy_bonus(&self, side: usize, mov: Move, params: &MctsParams) -> f32 {
        let divisor = params.butterfly_policy_divisor().max(1) as f32;
        f32::from(self.entry(side, mov).load(Ordering::Relaxed)) / divisor
    }

    fn clear(&self) {
        for entry in &self.data {
            entry.store(0, Ordering::Relaxed);
        }
    }

    fn update(&self, side: usize, mov: Move, score: f32, params: &MctsParams) {
        if !score.is_finite() {
            return;
        }

        let score = score.clamp(0.001, 0.999);
        let cp = (-400.0 * ((1.0 / score) - 1.0).ln()).round() as i32;
        let cell = self.entry(side, mov);

        let mut current = cell.load(Ordering::Relaxed);
        loop {
            let delta = scale_bonus(current, cp, params.butterfly_reduction_factor());
            let new = current.saturating_add(delta);
            match cell.compare_exchange(current, new, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }
}

fn scale_bonus(score: i16, bonus: i32, reduction_factor: i32) -> i16 {
    let bonus = bonus.clamp(i16::MIN as i32, i16::MAX as i32);
    let reduction_factor = reduction_factor.max(1);
    let reduction = i32::from(score) * bonus.abs() / reduction_factor;
    let adjusted = bonus - reduction;
    adjusted.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

pub struct Tree {
    root: ChessState,
    tree: [TreeHalf; 2],
    half: AtomicBool,
    hash: HashTable,
    butterfly: ButterflyTable,
    root_accumulator: RootAccumulator,
}

impl Index<NodePtr> for Tree {
    type Output = Node;

    fn index(&self, index: NodePtr) -> &Self::Output {
        &self.tree[usize::from(index.half())][index]
    }
}

impl Tree {
    pub fn new_mb(mb: usize, threads: usize) -> Self {
        let bytes = mb * 1024 * 1024;

        const _: () = assert!(
            std::mem::size_of::<Node>() == 64,
            "You must reconsider this allocation!"
        );

        let node_bytes = std::mem::size_of::<Node>() + 2;

        Self::new(bytes / node_bytes, bytes / node_bytes / 16, threads)
    }

    fn new(tree_cap: usize, hash_cap: usize, threads: usize) -> Self {
        let tree = Self {
            root: ChessState::default(),
            tree: [
                TreeHalf::new(tree_cap / 2, false, threads),
                TreeHalf::new(tree_cap / 2, true, threads),
            ],
            half: AtomicBool::new(false),
            hash: HashTable::new(hash_cap / 4, threads),
            butterfly: ButterflyTable::new(),
            root_accumulator: RootAccumulator::new(threads),
        };

        tree.reset_root_accumulator();

        tree
    }

    pub fn root_position(&self) -> &ChessState {
        &self.root
    }

    pub fn half(&self) -> usize {
        usize::from(self.half.load(Ordering::Relaxed))
    }

    pub fn is_full(&self) -> bool {
        self.tree[self.half()].is_full()
    }

    pub fn push_new_node(&self) -> Option<NodePtr> {
        self.tree[self.half()].reserve_nodes_thread(1, 0)
    }

    fn copy_node_across(&self, from: NodePtr, to: NodePtr) {
        if from == to {
            return;
        }

        let f = self[from].actions_mut();
        let t = self[to].actions_mut();

        self[to].copy_from(&self[from]);
        self[to].set_num_actions(self[from].num_actions());
        t.store(f.val());
    }

    fn copy_across(&self, from: NodePtr, num: usize, to: NodePtr) {
        for i in 0..num {
            self.copy_node_across(from + i, to + i);
        }
    }

    pub fn flip(&self, copy_across: bool, threads: usize) {
        let old_root_ptr = self.root_node();

        self.root_accumulator
            .flush_all(|ptr, delta| self[ptr].apply_delta(delta));

        let old = usize::from(self.half.fetch_xor(true, Ordering::Relaxed));
        self.tree[old].clear_ptrs(threads);
        self.tree[old ^ 1].clear();

        if copy_across {
            let new_root_ptr = self.tree[self.half()].reserve_nodes_thread(1, 0).unwrap();
            self[new_root_ptr].clear();

            self.copy_node_across(old_root_ptr, new_root_ptr);
        }

        self.reset_root_accumulator();
    }

    #[must_use]
    pub fn fetch_children(&self, parent_ptr: NodePtr, thread_id: usize) -> Option<()> {
        let first_child_ptr = { self[parent_ptr].actions() };

        if first_child_ptr.half() != self.half.load(Ordering::Relaxed) {
            let most_recent_ptr = self[parent_ptr].actions_mut();

            if most_recent_ptr.val().half() == self.half.load(Ordering::Relaxed) {
                return Some(());
            }

            assert_eq!(first_child_ptr, most_recent_ptr.val());

            let num_children = self[parent_ptr].num_actions();
            let new_ptr = self.tree[self.half()].reserve_nodes_thread(num_children, thread_id)?;

            self.copy_across(first_child_ptr, num_children, new_ptr);

            most_recent_ptr.store(new_ptr);
        }

        Some(())
    }

    pub fn root_node(&self) -> NodePtr {
        NodePtr::new(self.half.load(Ordering::Relaxed), 0)
    }

    pub fn probe_hash(&self, hash: u64) -> Option<HashEntry> {
        self.hash.get(hash)
    }

    pub fn push_hash(&self, hash: u64, wins: f32) {
        self.hash.push(hash, wins);
    }

    pub fn update_node_stats(&self, ptr: NodePtr, value: f32, thread_id: usize) {
        let delta = NodeStatsDelta::from_value(value);
        self.root_accumulator.add(ptr, &self[ptr], delta, thread_id);
    }

    pub fn flush_root_accumulator(&self) {
        self.root_accumulator
            .flush_all(|ptr, delta| self[ptr].apply_delta(delta));
    }

    fn reset_root_accumulator(&self) {
        self.root_accumulator.reset(self.root_node());
    }

    fn clear_halves(&self) {
        self.tree[0].clear();
        self.tree[1].clear();
    }

    pub fn clear(&mut self, threads: usize) {
        self.root = ChessState::default();
        self.clear_halves();
        self.hash.clear(threads);
        self.butterfly.clear();
        self.root_accumulator.reset(self.root_node());
    }

    pub fn is_empty(&self) -> bool {
        self.tree[0].is_empty() && self.tree[1].is_empty()
    }

    pub fn expand_node(
        &self,
        node_ptr: NodePtr,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
        depth: usize,
        thread_id: usize,
    ) -> Option<()> {
        let node = &self[node_ptr];

        let actions_ptr = node.actions_mut();

        // when running with >1 threads, this function may
        // be called twice, and this acts as a safeguard in
        // that case
        if !node.is_not_expanded() {
            return Some(());
        }

        let mut max = f32::NEG_INFINITY;
        let mut moves = [const { MaybeUninit::uninit() }; 256];
        let mut count = 0;
        let stm = pos.stm();

        pos.map_moves_with_policies(policy, |mov, policy| {
            let adjusted = policy + self.butterfly.policy_bonus(stm, mov, params);
            moves[count].write((mov, adjusted));
            count += 1;
            max = max.max(adjusted);
        });

        let new_ptr = self.tree[self.half()].reserve_nodes_thread(count, thread_id)?;

        let pst = SearchHelpers::get_pst(depth, self[node_ptr].q(), params);

        let slice = unsafe {
            std::slice::from_raw_parts_mut(moves.as_mut_ptr() as *mut (Move, f32), count)
        };

        let mut total = 0.0;
        for (_, policy) in slice.iter_mut() {
            *policy = ((*policy - max) / pst).exp();
            total += *policy;
        }

        slice.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut sum_of_squares = 0.0;

        for (action, (mov, policy)) in slice.iter().enumerate() {
            let ptr = new_ptr + action;
            let policy = policy / total;

            self[ptr].set_new(*mov, policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        node.set_gini_impurity(gini_impurity);

        actions_ptr.store(new_ptr);
        node.set_num_actions(count);

        Some(())
    }

    pub fn relabel_policy(
        &self,
        node_ptr: NodePtr,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
        depth: u8,
    ) {
        let actions = self[node_ptr].actions_mut();
        let num_actions = self[node_ptr].num_actions();
        let actions_ptr = actions.val();

        let hl = pos.get_policy_hl(policy);
        let mut max = f32::NEG_INFINITY;
        let mut policies = Vec::new();

        let stm = pos.stm();
        for action in 0..num_actions {
            let mov = self[actions_ptr + action].parent_move();
            let policy =
                pos.get_policy(mov, &hl, policy) + self.butterfly.policy_bonus(stm, mov, params);

            policies.push(policy);
            max = max.max(policy);
        }

        let pst = SearchHelpers::get_pst(depth.into(), self[node_ptr].q(), params);

        let mut total = 0.0;

        for policy in &mut policies {
            *policy = ((*policy - max) / pst).exp();
            total += *policy;
        }

        let mut sum_of_squares = 0.0;

        for (action, &policy) in policies.iter().enumerate() {
            let policy = policy / total;
            self[actions_ptr + action].set_policy(policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        self[node_ptr].set_gini_impurity(gini_impurity);
    }

    pub fn update_butterfly(&self, side: usize, mov: Move, score: f32, params: &MctsParams) {
        self.butterfly.update(side, mov, score, params);
    }

    pub fn clear_butterfly_table(&self) {
        self.butterfly.clear();
    }

    pub fn propogate_proven_mates(&self, ptr: NodePtr, child_state: GameState) {
        match child_state {
            // if the child node resulted in a loss, then
            // this node has a guaranteed win
            GameState::Lost(n) => self[ptr].set_state(GameState::Won(n + 1)),
            // if the child node resulted in a win, then check if there are
            // any non-won children, and if not, guaranteed loss for this node
            GameState::Won(n) => {
                assert_ne!(self[ptr].num_actions(), 0);

                let mut proven_loss = true;
                let mut max_win_len = n;
                let first_child_ptr = self[ptr].actions();

                for action in 0..self[ptr].num_actions() {
                    let ptr = first_child_ptr + action;

                    if let GameState::Won(n) = self[ptr].state() {
                        max_win_len = n.max(max_win_len);
                    } else {
                        proven_loss = false;
                        break;
                    }
                }

                if proven_loss {
                    self[ptr].set_state(GameState::Lost(max_win_len + 1));
                }
            }
            // nothing to do otherwise
            _ => {}
        }
    }

    pub fn set_root_position(&mut self, new_root: &ChessState) {
        let old_root = self.root.clone();
        self.root = new_root.clone();

        self.flush_root_accumulator();
        self.reset_root_accumulator();

        if self.is_empty() {
            return;
        }

        let mut found = false;

        println!("info string searching for subtree");

        let root = self.recurse_find(self.root_node(), &old_root, new_root, 2);

        if !root.is_null() && self[root].has_children() {
            found = true;

            if root != self.root_node() {
                self[self.root_node()].clear();
                self.copy_node_across(root, self.root_node());
            }

            println!("info string found subtree");
        }

        if !found {
            println!("info string no subtree found");
            self.clear_halves();
        }
    }

    fn recurse_find(
        &self,
        start: NodePtr,
        this_board: &ChessState,
        board: &ChessState,
        depth: u8,
    ) -> NodePtr {
        if this_board.board() == board.board() {
            return start;
        }

        if start.is_null() || depth == 0 {
            return NodePtr::NULL;
        }

        let first_child_ptr = self[start].actions();

        if first_child_ptr.is_null() {
            return NodePtr::NULL;
        }

        for action in 0..self[start].num_actions() {
            let mut child_board = this_board.clone();

            let child_ptr = first_child_ptr + action;
            let child = &self[child_ptr];

            child_board.make_move(child.parent_move());

            let found = self.recurse_find(child_ptr, &child_board, board, depth - 1);

            if !found.is_null() {
                return found;
            }
        }

        NodePtr::NULL
    }

    pub fn get_best_child(&self, ptr: NodePtr) -> usize {
        self.get_best_child_by_key(ptr, |n| n.visits() as f32)
    }

    pub fn get_best_child_by_key<F: FnMut(&Node) -> f32>(&self, ptr: NodePtr, key: F) -> usize {
        let limit = self[ptr].num_actions();
        self.get_best_child_by_key_lim(ptr, limit, key)
    }

    pub fn get_best_child_by_key_lim<F: FnMut(&Node) -> f32>(
        &self,
        ptr: NodePtr,
        limit: usize,
        mut key: F,
    ) -> usize {
        let mut best_child = usize::MAX;
        let mut best_score = f32::NEG_INFINITY;

        let first_child_ptr = self[ptr].actions();

        for action in 0..limit.min(self[ptr].num_actions()) {
            let score = key(&self[first_child_ptr + action]);

            if score > best_score {
                best_score = score;
                best_child = action;
            }
        }

        best_child
    }

    #[cfg(feature = "datagen")]
    pub fn get_best_child_temp(&self, ptr: NodePtr, temp: f32) -> Move {
        use rand::prelude::*;
        use rand_distr::Uniform;

        let node = &self[ptr];
        let child_ptr = node.actions();

        if temp == 0.0 {
            return self[child_ptr + self.get_best_child(ptr)].parent_move();
        }

        let mut rng = rand::rng();
        let dist = Uniform::new(0.0, 1.0).unwrap();
        let rand = dist.sample(&mut rng);

        let mut total = 0.0;
        let mut distribution = vec![0.0; node.num_actions()];
        let t = 1.0 / f64::from(temp);

        for i in 0..node.num_actions() {
            let child = &self[child_ptr + i];
            distribution[i] = (child.visits() as f64).powf(t);
            total += distribution[i];
        }

        let mut cumulative = 0.0;

        for (i, weight) in distribution.iter().enumerate() {
            cumulative += weight;

            if cumulative / total > rand {
                return self[child_ptr + i].parent_move();
            }
        }

        self[child_ptr + (node.num_actions() - 1)].parent_move()
    }

    #[cfg(feature = "datagen")]
    pub fn add_dirichlet_noise_to_node(&self, ptr: NodePtr, alpha: f32, prop: f32) {
        use rand_distr::{Distribution, Gamma};

        let node = &self[ptr];

        if node.num_actions() <= 1 {
            return;
        }

        let actions_ptr = node.actions();

        let mut rng = rand::rng();
        let k = node.num_actions();

        // Symmetric Dirichlet via Gamma(alpha, 1) samples
        let gamma = Gamma::<f32>::new(alpha, 1.0).unwrap();
        let mut sum = 0.0;
        let mut noise = Vec::with_capacity(k);
        for _ in 0..k {
            let x = gamma.sample(&mut rng);
            sum += x;
            noise.push(x);
        }
        // Guard against pathological underflow
        let inv_sum = if sum > 0.0 { 1.0 / sum } else { 1.0 / k as f32 };

        for (action, x) in noise.into_iter().enumerate() {
            let child = &self[actions_ptr + action];
            let mixed = (1.0 - prop) * child.policy() + prop * (x * inv_sum);
            child.set_policy(mixed);
        }
    }
}
