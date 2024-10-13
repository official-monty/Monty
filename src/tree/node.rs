use std::sync::{
    atomic::{AtomicU16, AtomicU32, Ordering},
    RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use crate::{
    chess::Move, mcts::SearchHelpers, tree::{Edge, NodePtr}, ChessState, GameState, MctsParams, PolicyNetwork
};

use super::ActionStats;

#[derive(Debug)]
pub struct Node {
    actions: RwLock<Vec<Edge>>,
    state: AtomicU16,
    threads: AtomicU16,

    // heuristics used in search
    gini_impurity: AtomicU32,
}

impl Node {
    pub fn new(state: GameState) -> Self {
        Node {
            actions: RwLock::new(Vec::new()),
            state: AtomicU16::new(u16::from(state)),
            threads: AtomicU16::new(0),
            gini_impurity: AtomicU32::new(0),
        }
    }

    pub fn set_new(&self, state: GameState) {
        *self.actions_mut() = Vec::new();
        self.set_state(state);
        self.set_gini_impurity(0.0);
    }

    pub fn is_terminal(&self) -> bool {
        self.state() != GameState::Ongoing
    }

    pub fn num_actions(&self) -> usize {
        self.actions.read().unwrap().len()
    }

    pub fn threads(&self) -> u16 {
        self.threads.load(Ordering::Relaxed)
    }

    pub fn inc_threads(&self) {
        self.threads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_threads(&self) {
        self.threads.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn actions(&self) -> RwLockReadGuard<Vec<Edge>> {
        self.actions.read().unwrap()
    }

    pub fn actions_mut(&self) -> RwLockWriteGuard<Vec<Edge>> {
        self.actions.write().unwrap()
    }

    pub fn state(&self) -> GameState {
        GameState::from(self.state.load(Ordering::Relaxed))
    }

    pub fn set_state(&self, state: GameState) {
        self.state.store(u16::from(state), Ordering::Relaxed);
    }

    pub fn has_children(&self) -> bool {
        self.actions.read().unwrap().len() != 0
    }

    pub fn is_not_expanded(&self) -> bool {
        self.state() == GameState::Ongoing && !self.has_children()
    }

    pub fn gini_impurity(&self) -> f32 {
        f32::from_bits(self.gini_impurity.load(Ordering::Relaxed))
    }

    pub fn set_gini_impurity(&self, gini_impurity: f32) {
        self.gini_impurity
            .store(f32::to_bits(gini_impurity), Ordering::Relaxed);
    }

    pub fn clear(&self) {
        *self.actions.write().unwrap() = Vec::new();
        self.set_state(GameState::Ongoing);
        self.set_gini_impurity(0.0);
    }

    pub fn expand(
        &self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
        parent_stats: &ActionStats,
        depth: usize,
    ) {
        let mut actions = self.actions_mut();

        if actions.len() != 0 {
            return;
        }

        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats, policy);

            // trick for calculating policy before quantising
            actions.push(Edge::new(
                NodePtr::from_raw(f32::to_bits(policy)),
                mov.into(),
                0,
            ));
            max = max.max(policy);
        });

        let pst = match depth {
            0 => unreachable!(),
            1 => params.root_pst(),
            2 => params.depth_2_pst(),
            3.. => SearchHelpers::get_pst(parent_stats, params),
        };

        let mut total = 0.0;

        for action in actions.iter_mut() {
            let mut policy = f32::from_bits(action.ptr().inner());

            policy = ((policy - max) / pst).exp();

            action.set_ptr(NodePtr::from_raw(f32::to_bits(policy)));

            total += policy;
        }

        let mut sum_of_squares = 0.0;

        for action in actions.iter_mut() {
            let policy = f32::from_bits(action.ptr().inner()) / total;
            action.set_ptr(NodePtr::NULL);
            action.set_policy(policy);
            sum_of_squares += policy * policy;
        }

        let gini_impurity = (1.0 - sum_of_squares).clamp(0.0, 1.0);
        self.set_gini_impurity(gini_impurity);
    }

    pub fn relabel_policy(
        &self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
        depth: u8,
    ) {
        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        let mut policies = Vec::new();

        for action in self.actions().iter() {
            let mov = Move::from(action.mov());
            let policy = pos.get_policy(mov, &feats, policy);
            policies.push(policy);
            max = max.max(policy);
        }

        let pst = match depth {
            0 => unreachable!(),
            1 => params.root_pst(),
            2 => params.depth_2_pst(),
            3.. => unreachable!(),
        };

        let mut total = 0.0;

        for policy in &mut policies {
            *policy = ((*policy - max) / pst).exp();
            total += *policy;
        }

        for (i, action) in self.actions_mut().iter_mut().enumerate() {
            action.set_policy(policies[i] / total);
        }
    }
}
