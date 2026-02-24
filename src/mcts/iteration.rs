use crate::{
    chess::{ChessState, GameState},
    tree::{Node, NodePtr},
};

use super::{SearchHelpers, Searcher};

pub fn perform_one(
    searcher: &Searcher,
    pos: &mut ChessState,
    ptr: NodePtr,
    depth: &mut usize,
    root_child: &mut Option<NodePtr>,
    thread_id: usize,
) -> Option<(f32, f32)> {
    *depth += 1;

    let cur_hash = pos.hash();
    let mut child_hash: Option<u64> = None;
    let mut child_visits = 0;
    let tree = searcher.tree;
    let node = &tree[ptr];

    let mut value = if node.is_terminal() || node.visits() == 0 {
        if node.visits() == 0 {
            node.set_state(pos.game_state());
        }

        // probe hash table to use in place of network
        if node.state() == GameState::Ongoing {
            if let Some(entry) = tree.probe_hash(cur_hash) {
                (entry.q(), entry.d())
            } else {
                get_utility(searcher, ptr, pos)
            }
        } else {
            get_utility(searcher, ptr, pos)
        }
    } else {
        // expand node on the second visit
        if node.is_not_expanded() {
            tree.expand_node(
                ptr,
                pos,
                searcher.params,
                searcher.policy,
                *depth,
                thread_id,
            )?;
        }

        // this node has now been accessed so we need to move its
        // children across if they are in the other tree half
        tree.fetch_children(ptr, thread_id)?;

        // select action to take via PUCT
        let stm = pos.stm();
        let action = pick_action(searcher, ptr, node);

        let child_ptr = node.actions() + action;
        if ptr == searcher.tree.root_node() {
            *root_child = Some(child_ptr);
        }

        let mov = tree[child_ptr].parent_move();

        pos.make_move(mov);

        // capture child hash (value is stored from the side to move at this child)
        child_hash = Some(pos.hash());

        child_visits = tree[child_ptr].visits();
        tree[child_ptr].inc_threads();

        // acquire lock to avoid issues with desynced setting of
        // game state between threads when threads > 1
        let lock = if tree[child_ptr].visits() == 0 {
            Some(node.actions_mut())
        } else {
            None
        };

        // descend further
        let maybe_u = perform_one(searcher, pos, child_ptr, depth, root_child, thread_id);

        drop(lock);

        tree[child_ptr].dec_threads();

        let u = maybe_u?;

        if tree[child_ptr].state() == GameState::Ongoing {
            tree.update_butterfly(stm, mov, u.0, searcher.params);
        }

        tree.propogate_proven_mates(ptr, tree[child_ptr].state());

        u
    };

    // store value for the side to move at the visited node in TT
    if let Some(h) = child_hash {
        // `u` here is from the current node's perspective, so flip for the child
        tree.push_hash(h, 1.0 - value.0, value.1, child_visits);
    } else {
        tree.push_hash(cur_hash, value.0, value.1, 1);
    }

    // flip perspective and backpropagate
    value.0 = 1.0 - value.0;
    tree.update_node_stats(ptr, value.0, value.1, thread_id);
    Some(value)
}

fn get_utility(searcher: &Searcher, ptr: NodePtr, pos: &ChessState) -> (f32, f32) {
    match searcher.tree[ptr].state() {
        GameState::Ongoing => {
            let eval = pos.eval_with_contempt(
                searcher.value,
                searcher.params,
                searcher.tree.root_position().stm(),
            );
            (eval.contempt.score(), eval.contempt.draw)
        }
        GameState::Draw => (0.5, 1.0),
        GameState::Lost(_) => (0.0, 0.0),
        GameState::Won(_) => (1.0, 0.0),
    }
}

fn pick_action(searcher: &Searcher, ptr: NodePtr, node: &Node) -> usize {
    let is_root = ptr == searcher.tree.root_node();

    let cpuct = SearchHelpers::get_cpuct(searcher.params, node, is_root);
    let fpu = SearchHelpers::get_fpu(node);
    let expl_scale = SearchHelpers::get_explore_scaling(searcher.params, node);

    let expl = cpuct * expl_scale;

    let actions_ptr = node.actions();
    let mut acc = 0.0;
    let mut k = 0;
    while k < node.num_actions() && acc < searcher.params.policy_top_p() {
        acc += searcher.tree[actions_ptr + k].policy();
        k += 1;
    }
    let mut limit = k.max(searcher.params.min_policy_actions() as usize);
    let mut thresh = 1u64 << (searcher.params.visit_threshold_power() as u32);
    while node.visits() >= thresh && limit < node.num_actions() {
        limit += 2;
        thresh = thresh.checked_shl(1).unwrap_or(u64::MAX);
    }
    limit = limit.min(node.num_actions());

    let posterior_policy = (searcher.params.rmcts_enable() != 0)
        .then(|| compute_rmcts_policy(searcher, node, actions_ptr, limit, fpu));

    let mut best_action = 0;
    let mut best_score = f32::NEG_INFINITY;

    for action in 0..limit {
        let child = &searcher.tree[actions_ptr + action];
        let mut q = SearchHelpers::get_action_value(child, fpu);

        let threads = f64::from(child.threads());
        if threads > 0.0 {
            let visits = child.visits() as f64;
            let q2 = f64::from(q) * visits
                / (visits + 1.0 + searcher.params.virtual_loss_weight() * (threads - 1.0));
            q = q2 as f32;
        }

        let policy = posterior_policy
            .as_ref()
            .map_or_else(|| child.policy(), |pi| pi[action]);
        let u = expl * policy / (1 + child.visits()) as f32;

        let score = q + u;
        if score > best_score {
            best_score = score;
            best_action = action;
        }
    }

    best_action
}

fn compute_rmcts_policy(
    searcher: &Searcher,
    node: &Node,
    actions_ptr: NodePtr,
    limit: usize,
    fpu: f32,
) -> Vec<f32> {
    let mut q_max = f32::NEG_INFINITY;
    let mut priors = Vec::with_capacity(limit);
    let mut values = Vec::with_capacity(limit);

    for action in 0..limit {
        let child = &searcher.tree[actions_ptr + action];
        priors.push(child.policy().max(1e-8));
        let q = SearchHelpers::get_action_value(child, fpu);
        values.push(q);
        q_max = q_max.max(q);
    }

    let sum_prior: f32 = priors.iter().sum();
    if sum_prior <= 0.0 || !sum_prior.is_finite() {
        return vec![1.0 / limit as f32; limit];
    }
    for p in &mut priors {
        *p /= sum_prior;
    }

    let t = node.visits().max(1) as f32;
    let c0 = searcher.params.rmcts_c() / t.sqrt();
    let mut delta = (c0 * priors.iter().copied().fold(0.0, f32::max)).max(1e-8);

    for _ in 0..16 {
        let mut f = -1.0f32;
        let mut fprime = 0.0f32;
        for (&pi0, &q) in priors.iter().zip(values.iter()) {
            let denom = (q_max - q + delta).max(1e-8);
            let x = c0 * pi0 / denom;
            f += x;
            fprime -= x / denom;
        }

        if f <= 1e-6 {
            break;
        }

        let new_delta = delta - f / fprime.min(-1e-8);
        if !new_delta.is_finite() || new_delta <= 0.0 {
            break;
        }
        delta = new_delta;
    }

    let mut posterior = Vec::with_capacity(limit);
    let mut sum = 0.0;
    for (&pi0, &q) in priors.iter().zip(values.iter()) {
        let p = c0 * pi0 / (q_max - q + delta).max(1e-8);
        posterior.push(p);
        sum += p;
    }

    if sum > 0.0 && sum.is_finite() {
        for p in &mut posterior {
            *p /= sum;
        }
    } else {
        posterior.fill(1.0 / limit as f32);
    }

    posterior
}
