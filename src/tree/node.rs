use std::sync::{atomic::{AtomicI32, AtomicU16, Ordering}, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{chess::Move, tree::Edge, ChessState, GameState, MctsParams, PolicyNetwork};

#[derive(Debug)]
pub struct Node {
    actions: RwLock<Vec<Edge>>,
    state: AtomicU16,

    // used for lru
    bwd_link: AtomicI32,
    fwd_link: AtomicI32,
    parent: AtomicI32,
    action: AtomicU16,
}

impl Node {
    pub fn new(state: GameState, parent: i32, action: usize) -> Self {
        Node {
            actions: RwLock::new(Vec::new()),
            state: AtomicU16::new(u16::from(state)),
            parent: AtomicI32::new(parent),
            bwd_link: AtomicI32::new(-1),
            fwd_link: AtomicI32::new(-1),
            action: AtomicU16::new(action as u16),
        }
    }

    pub fn set_new(&self, state: GameState, parent: i32, action: usize) {
        self.clear();
        self.state.store(u16::from(state), Ordering::Relaxed);
        self.parent.store(parent, Ordering::Relaxed);
        self.action.store(action as u16, Ordering::Relaxed);
    }

    pub fn parent(&self) -> i32 {
        self.parent.load(Ordering::Relaxed)
    }

    pub fn is_terminal(&self) -> bool {
        self.state() != GameState::Ongoing
    }

    pub fn num_actions(&self) -> usize {
        self.actions.read().unwrap().len()
    }

    pub fn actions(&self) -> RwLockReadGuard<Vec<Edge>> {
        self.actions.read().unwrap()
    }

    fn actions_mut(&self) -> RwLockWriteGuard<Vec<Edge>> {
        self.actions.write().unwrap()
    }

    pub fn state(&self) -> GameState {
        GameState::from(self.state.load(Ordering::Relaxed))
    }

    pub fn bwd_link(&self) -> i32 {
        self.bwd_link.load(Ordering::Relaxed)
    }

    pub fn fwd_link(&self) -> i32 {
        self.fwd_link.load(Ordering::Relaxed)
    }

    pub fn set_state(&self, state: GameState) {
        self.state.store(u16::from(state), Ordering::Relaxed);
    }

    pub fn has_children(&self) -> bool {
        self.actions.read().unwrap().len() != 0
    }

    pub fn action(&self) -> usize {
        usize::from(self.action.load(Ordering::Relaxed))
    }

    pub fn clear_parent(&self) {
        self.parent.store(-1, Ordering::Relaxed);
        self.action.store(0, Ordering::Relaxed);
    }

    pub fn is_not_expanded(&self) -> bool {
        self.state() == GameState::Ongoing && !self.has_children()
    }

    pub fn clear(&self) {
        self.actions.write().unwrap().clear();
        self.set_state(GameState::Ongoing);
        self.set_bwd_link(-1);
        self.set_fwd_link(-1);
    }

    pub fn set_fwd_link(&self, ptr: i32) {
        self.fwd_link.store(ptr, Ordering::Relaxed);
    }

    pub fn set_bwd_link(&self, ptr: i32) {
        self.bwd_link.store(ptr, Ordering::Relaxed);
    }

    pub fn expand<const ROOT: bool>(
        &self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
    ) {
        assert!(self.is_not_expanded());

        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        let mut actions = self.actions_mut();

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats, policy);

            // trick for calculating policy before quantising
            actions.push(Edge::new(f32::to_bits(policy) as i32, mov.into(), 0));
            max = max.max(policy);
        });

        let mut total = 0.0;

        for action in actions.iter_mut()  {
            let mut policy = f32::from_bits(action.ptr() as u32);

            policy = if ROOT {
                ((policy - max) / params.root_pst()).exp()
            } else {
                (policy - max).exp()
            };

            action.set_ptr(f32::to_bits(policy) as i32);

            total += policy;
        }

        for action in actions.iter_mut() {
            let policy = f32::from_bits(action.ptr() as u32) / total;
            action.set_ptr(-1);
            action.set_policy(policy);
        }
    }

    pub fn relabel_policy(
        &self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
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

        let mut total = 0.0;

        for policy in &mut policies {
            *policy = ((*policy - max) / params.root_pst()).exp();
            total += *policy;
        }

        for (i, action) in self.actions_mut().iter_mut().enumerate() {
            action.set_policy(policies[i] / total);
        }
    }
}
