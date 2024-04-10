use crate::{GameState, tree::Edge, MctsParams, GameRep};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Empty,
    Var1,
    Var2,
}

impl Mark {
    pub fn flip(self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Var1 => Self::Var2,
            Self::Var2 => Self::Var1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    actions: Vec<Edge>,
    state: GameState,
    mark: Mark,
    fwd_link: i32,
    visits: i32,
    wins: f32,
}

impl Node {
    pub fn new(state: GameState) -> Self {
        Node {
            actions: Vec::new(),
            state,
            mark: Mark::Empty,
            fwd_link: -1,
            visits: 0,
            wins: 0.0,
        }
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

    pub fn mark(&self) -> Mark {
        self.mark
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

    pub fn visits(&self) -> i32 {
        self.visits
    }

    pub fn q(&self) -> f32 {
        match self.state {
            GameState::Won(_) => 0.0,
            GameState::Lost(_) => 1.0,
            GameState::Draw => 0.5,
            GameState::Ongoing => self.wins / self.visits as f32,
        }
    }

    pub fn is_not_expanded(&self) -> bool {
        self.state == GameState::Ongoing && self.actions.is_empty()
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.state = GameState::Ongoing;
        self.mark = Mark::Empty;
        self.fwd_link = -1;
        self.visits = 0;
        self.wins = 0.0;
    }

    pub fn set_mark(&mut self, mark: Mark) {
        self.mark = mark;
    }

    pub fn set_fwd_link(&mut self, ptr: i32) {
        self.fwd_link = ptr;
    }

    pub fn update(&mut self, visits: i32, result: f32) {
        self.visits += visits;
        self.wins += result;
    }

    pub fn expand<T: GameRep, const ROOT: bool>(
        &mut self,
        pos: &T,
        params: &MctsParams,
    ) {
        assert!(self.is_not_expanded());

        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        pos.map_legal_moves(|mov| {
            let policy = pos.get_policy(mov, &feats);

            // trick for calculating policy before quantising
            self.actions.push(Edge::new(f32::to_bits(policy) as i32, mov.into(), 0));
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

    pub fn relabel_policy<T: GameRep>(&mut self, pos: &T, params: &MctsParams) {
        let feats = pos.get_policy_feats();
        let mut max = f32::NEG_INFINITY;

        let mut policies = Vec::new();

        for action in &self.actions {
            let mov = T::Move::from(action.mov());
            let policy = pos.get_policy(mov, &feats);
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