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
    thread_id: usize,
) -> Option<f32> {
    *depth += 1;

    let cur_hash = pos.hash();
    let mut child_hash: Option<u64> = None;
    let tree = searcher.tree;
    let node = &tree[ptr];

    let mut u = if node.is_terminal() || node.visits() == 0 {
        if node.visits() == 0 {
            node.set_state(pos.game_state());
        }

        // probe hash table to use in place of network
        if node.state() == GameState::Ongoing {
            if let Some(entry) = tree.probe_hash(cur_hash) {
                entry.q()
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
        let action = pick_action(searcher, ptr, node);

        let child_ptr = node.actions() + action;

        let mov = tree[child_ptr].parent_move();

        pos.make_move(mov);

        // capture child hash (value is stored from the side to move at this child)
        child_hash = Some(pos.hash());

        tree[child_ptr].inc_threads();

        // acquire lock to avoid issues with desynced setting of
        // game state between threads when threads > 1
        let lock = if tree[child_ptr].visits() == 0 {
            Some(node.actions_mut())
        } else {
            None
        };

        // descend further
        let maybe_u = perform_one(searcher, pos, child_ptr, depth, thread_id);

        drop(lock);

        tree[child_ptr].dec_threads();

        let u = maybe_u?;

        tree.propogate_proven_mates(ptr, tree[child_ptr].state());

        u
    };

    // store value for the side to move at the visited node in TT
    if let Some(h) = child_hash {
        // `u` here is from the current node's perspective, so flip for the child
        tree.push_hash(h, 1.0 - u);
    } else {
        tree.push_hash(cur_hash, u);
    }

    // flip perspective and backpropagate
    u = 1.0 - u;
    node.update(u);
    Some(u)
}

fn get_utility(searcher: &Searcher, ptr: NodePtr, pos: &ChessState) -> f32 {
    match searcher.tree[ptr].state() {
        GameState::Ongoing => pos.get_value_wdl(searcher.value, searcher.params),
        GameState::Draw => 0.5,
        GameState::Lost(_) => 0.0,
        GameState::Won(_) => 1.0,
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
    while k < node.num_actions() && acc < 0.7 {
        acc += searcher.tree[actions_ptr + k].policy();
        k += 1;
    }
    let mut limit = k.max(6);
    let mut thresh = 1 << 3; //8
    while node.visits() >= thresh && limit < node.num_actions() {
        limit += 2;
        thresh <<= 1;
    }
    limit = limit.min(node.num_actions());

    searcher
        .tree
        .get_best_child_by_key_lim(ptr, limit, |child| {
        let mut q = SearchHelpers::get_action_value(child, fpu);

        // virtual loss
        let threads = f64::from(child.threads());
        if threads > 0.0 {
            let visits = f64::from(child.visits());
            let q2 = f64::from(q) * visits
                / (visits + 1.0 + searcher.params.virtual_loss_weight() * (threads - 1.0));
            q = q2 as f32;
        }

        let u = expl * child.policy() / (1 + child.visits()) as f32;

        q + u
    })
}
