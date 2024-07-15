use std::{
    alloc::{self, Layout},
    sync::atomic::{AtomicI32, AtomicPtr, AtomicU16, AtomicU64, Ordering}
};

use crate::{chess::Move, tree::Edge, ChessState, GameState, MctsParams, PolicyNetwork};

const EDGE_SIZE: usize = std::mem::size_of::<Edge>();
const EDGE_ALIGN: usize = std::mem::align_of::<Edge>();

#[derive(Debug)]
pub struct Node {
    actions: AtomicPtr<Edge>,
    num_actions: AtomicU16,
    state: AtomicU16,
    hash: AtomicU64,

    // used for lru
    bwd_link: AtomicI32,
    fwd_link: AtomicI32,
    parent: AtomicI32,
    action: AtomicU16,
}

impl Node {
    pub fn new(state: GameState, hash: u64, parent: i32, action: usize) -> Self {
        Node {
            actions: AtomicPtr::new(std::ptr::null_mut()),
            num_actions: AtomicU16::new(0),
            state: AtomicU16::new(u16::from(state)),
            hash: AtomicU64::new(hash),
            parent: AtomicI32::new(parent),
            bwd_link: AtomicI32::new(-1),
            fwd_link: AtomicI32::new(-1),
            action: AtomicU16::new(action as u16),
        }
    }

    pub fn set_new(&self, state: GameState, hash: u64, parent: i32, action: usize) {
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

    pub fn num_actions(&self) -> usize {
        usize::from(self.num_actions.load(Ordering::Relaxed))
    }

    pub fn actions(&self) -> &[Edge] {
        let ptr = self.actions.load(Ordering::Relaxed);

        if ptr.is_null() {
            return &[];
        }

        unsafe {
            std::slice::from_raw_parts(ptr, self.num_actions())
        }
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
        !self.actions.load(Ordering::Relaxed).is_null()
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
        let ptr = self.actions.load(Ordering::Relaxed);
        let layout = Layout::from_size_align(EDGE_SIZE * self.num_actions(), EDGE_ALIGN).unwrap();
        unsafe {
            alloc::dealloc(ptr.cast(), layout);
        }

        self.actions.store(std::ptr::null_mut(), Ordering::Relaxed);
        self.num_actions.store(0, Ordering::Relaxed);
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
        &self,
        pos: &ChessState,
        params: &MctsParams,
        policy: &PolicyNetwork,
    ) {
        assert!(self.is_not_expanded());

        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;
        let mut moves = [(0, 0.0); 256];
        let mut num = 0;

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats, policy);
            max = max.max(policy);
            moves[num] = (mov.into(), policy);
            num += 1;
        });

        let mut total = 0.0;

        for (_, policy) in moves[..num].iter_mut() {
            *policy = if ROOT {
                ((*policy - max) / params.root_pst()).exp()
            } else {
                (*policy - max).exp()
            };

            total += *policy;
        }

        if num != 0 {
            let layout = Layout::from_size_align(EDGE_SIZE * num, EDGE_ALIGN).unwrap();
            let ptr = unsafe { alloc::alloc_zeroed(layout) };

            self.num_actions.store(num as u16, Ordering::Relaxed);
            self.actions.store(ptr.cast(), Ordering::Release);
        }

        for (action, &(mov, policy)) in self.actions().iter().zip(moves[..num].iter()) {
            action.set_new(mov, policy / total);
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

        for action in self.actions() {
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

        for (i, action) in self.actions().iter().enumerate() {
            action.set_policy(policies[i] / total);
        }
    }
}
