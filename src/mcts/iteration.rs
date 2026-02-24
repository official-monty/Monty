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

    let posterior_weight = searcher.params.rmcts_posterior_weight();
    let posterior = if posterior_weight > 0.0 {
        Some(compute_rmcts_posterior(
            searcher,
            actions_ptr,
            limit,
            expl,
            fpu,
            node.visits(),
        ))
    } else {
        None
    };

    let mut best_idx = 0;
    let mut best_score = f32::NEG_INFINITY;

    for action in 0..limit {
        let child = &searcher.tree[actions_ptr + action];
        let mut q = SearchHelpers::get_action_value(child, fpu);

        // virtual loss
        let threads = f64::from(child.threads());
        if threads > 0.0 {
            let visits = child.visits() as f64;
            let q2 = f64::from(q) * visits
                / (visits + 1.0 + searcher.params.virtual_loss_weight() * (threads - 1.0));
            q = q2 as f32;
        }

        let prior = if let Some(ref posterior) = posterior {
            (1.0 - posterior_weight) * child.policy() + posterior_weight * posterior[action]
        } else {
            child.policy()
        };

        let u = expl * prior / (1 + child.visits()) as f32;
        let score = q + u;

        if score > best_score {
            best_score = score;
            best_idx = action;
        }
    }

    best_idx
}

fn compute_rmcts_posterior(
    searcher: &Searcher,
    actions_ptr: NodePtr,
    limit: usize,
    cpuct: f32,
    fpu: f32,
    visits: u64,
) -> Vec<f32> {
    let lambda = (cpuct / ((visits.max(1) as f32).sqrt())).max(1e-4);

    let mut priors = vec![0.0f32; limit];
    let mut q_vals = vec![0.0f32; limit];
    let mut prior_sum = 0.0f32;

    for action in 0..limit {
        let child = &searcher.tree[actions_ptr + action];
        let p = child.policy().max(1e-8);
        priors[action] = p;
        prior_sum += p;
        q_vals[action] = SearchHelpers::get_action_value(child, fpu);
    }

    if prior_sum <= 0.0 {
        return vec![1.0 / limit as f32; limit];
    }

    for p in &mut priors {
        *p /= prior_sum;
    }

    let q_max = q_vals
        .iter()
        .fold(f32::NEG_INFINITY, |acc, &q| if q > acc { q } else { acc });

    let target = 1.0 / lambda;
    let mut lo = q_max + 1e-4;
    let mut hi = lo + 1.0;

    while priors
        .iter()
        .zip(q_vals.iter())
        .map(|(&p, &q)| p / (hi - q).max(1e-6))
        .sum::<f32>()
        > target
    {
        hi *= 2.0;
    }

    for _ in 0..28 {
        let mid = 0.5 * (lo + hi);
        let denom_sum = priors
            .iter()
            .zip(q_vals.iter())
            .map(|(&p, &q)| p / (mid - q).max(1e-6))
            .sum::<f32>();

        if denom_sum > target {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let alpha = hi;
    let mut posterior: Vec<f32> = priors
        .iter()
        .zip(q_vals.iter())
        .map(|(&p, &q)| lambda * p / (alpha - q).max(1e-6))
        .collect();
    let z: f32 = posterior.iter().sum();

    if z > 0.0 {
        for prob in &mut posterior {
            *prob /= z;
        }
    } else {
        return vec![1.0 / limit as f32; limit];
    }

    posterior
}
