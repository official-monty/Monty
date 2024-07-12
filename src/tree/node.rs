use crate::{chess::Move, tree::Edge, ChessState, GameState, MctsParams, PolicyNetwork};

#[derive(Clone, Debug)]
pub struct Node {
    actions: Vec<Edge>,
    state: GameState,
    hash: u64,

    // used for lru
    bwd_link: i32,
    fwd_link: i32,
    parent: i32,
    action: u16,
}

impl Node {
    pub fn new(state: GameState, hash: u64, parent: i32, action: usize) -> Self {
        Node {
            actions: Vec::new(),
            state,
            hash,
            parent,
            bwd_link: -1,
            fwd_link: -1,
            action: action as u16,
        }
    }

    pub fn parent(&self) -> i32 {
        self.parent
    }

    pub fn is_terminal(&self) -> bool {
        self.state != GameState::Ongoing
    }

    pub fn actions(&self) -> &[Edge] {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut [Edge] {
        &mut self.actions
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn bwd_link(&self) -> i32 {
        self.bwd_link
    }

    pub fn fwd_link(&self) -> i32 {
        self.fwd_link
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn has_children(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn action(&self) -> usize {
        usize::from(self.action)
    }

    pub fn clear_parent(&mut self) {
        self.parent = -1;
        self.action = 0;
    }

    pub fn is_not_expanded(&self) -> bool {
        self.state == GameState::Ongoing && self.actions.is_empty()
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.state = GameState::Ongoing;
        self.hash = 0;
        self.bwd_link = -1;
        self.fwd_link = -1;
    }

    pub fn set_fwd_link(&mut self, ptr: i32) {
        self.fwd_link = ptr;
    }

    pub fn set_bwd_link(&mut self, ptr: i32) {
        self.bwd_link = ptr;
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

    #[cfg(feature = "datagen")]
    pub fn add_dirichlet_noise(&mut self, alpha: f32, prop: f32) {
        use rand::prelude::*;
        use rand_distr::Dirichlet;

        if self.actions.len() <= 1 {
            return;
        }

        let mut rng = rand::thread_rng();
        let dist = Dirichlet::new(&vec![alpha; self.actions.len()]).unwrap();
        let samples = dist.sample(&mut rng);

        for (action, &noise) in self.actions.iter_mut().zip(samples.iter()) {
            let policy = (1.0 - prop) * action.policy() + prop * noise;

            action.set_policy(policy);
        }
    }
}
