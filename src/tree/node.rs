use std::sync::atomic::{AtomicI32, AtomicU16, AtomicU64, Ordering};

use crate::{chess::Move, tree::Edge, ChessState, GameState, MctsParams, PolicyNetwork};

#[derive(Debug)]
pub struct Node {
    actions: Vec<Edge>,
    state: AtomicU16,
    hash: AtomicU64,

    // used for lru
    bwd_link: AtomicI32,
    fwd_link: AtomicI32,
    parent: AtomicI32,
    action: AtomicU16,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            actions: self.actions.clone(),
            state: AtomicU16::new(self.state.load(Ordering::Relaxed)),
            hash: AtomicU64::new(self.hash()),
            bwd_link: AtomicI32::new(self.bwd_link()),
            fwd_link: AtomicI32::new(self.fwd_link()),
            parent: AtomicI32::new(self.parent()),
            action: AtomicU16::new(self.action.load(Ordering::Relaxed)),
        }
    }
}

impl Node {
    pub fn new(state: GameState, hash: u64, parent: i32, action: usize) -> Self {
        Node {
            actions: Vec::new(),
            state: AtomicU16::new(u16::from(state)),
            hash: AtomicU64::new(hash),
            parent: AtomicI32::new(parent),
            bwd_link: AtomicI32::new(-1),
            fwd_link: AtomicI32::new(-1),
            action: AtomicU16::new(action as u16),
        }
    }

    pub fn set_new(&mut self, state: GameState, hash: u64, parent: i32, action: usize) {
        self.clear();
        self.state.store(u16::from(state), Ordering::Relaxed);
        self.hash.store(hash, Ordering::Relaxed);
        self.parent.store(parent, Ordering::Relaxed);
        self.action.store(action as u16, Ordering::Relaxed);
    }

    pub fn parent(&self) -> i32 {
        self.parent.load(Ordering::Relaxed)
    }

    pub fn is_terminal(&self) -> bool {
        self.state() != GameState::Ongoing
    }

    pub fn actions(&self) -> &[Edge] {
        &self.actions
    }

    pub fn state(&self) -> GameState {
        GameState::from(self.state.load(Ordering::Relaxed))
    }

    pub fn hash(&self) -> u64 {
        self.hash.load(Ordering::Relaxed)
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
        !self.actions.is_empty()
    }

    pub fn action(&self) -> usize {
        usize::from(self.action.load(Ordering::Relaxed))
    }

    pub fn clear_parent(&self) {
        self.parent.store(-1, Ordering::Relaxed);
        self.action.store(0, Ordering::Relaxed);
    }

    pub fn is_not_expanded(&self) -> bool {
        self.state() == GameState::Ongoing && self.actions.is_empty()
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.set_state(GameState::Ongoing);
        self.hash.store(0, Ordering::Relaxed);
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
        &mut self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
    ) {
        assert!(self.is_not_expanded());

        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats, policy);

            // trick for calculating policy before quantising
            self.actions
                .push(Edge::new(f32::to_bits(policy) as i32, mov.into(), 0));
            max = max.max(policy);
        });

        let mut total = 0.0;

        for action in &mut self.actions {
            let mut policy = f32::from_bits(action.ptr() as u32);

            policy = if ROOT {
                ((policy - max) / params.root_pst()).exp()
            } else {
                (policy - max).exp()
            };

            action.set_ptr(f32::to_bits(policy) as i32);

            total += policy;
        }

        for action in &mut self.actions {
            let policy = f32::from_bits(action.ptr() as u32) / total;
            action.set_ptr(-1);
            action.set_policy(policy);
        }
    }

    pub fn relabel_policy(
        &mut self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
    ) {
        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        let mut policies = Vec::new();

        for action in &self.actions {
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

        for (i, action) in self.actions.iter_mut().enumerate() {
            action.set_policy(policies[i] / total);
        }
    }
}
