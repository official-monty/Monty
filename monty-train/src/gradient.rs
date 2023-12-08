use crate::TrainingPosition;

use monty_core::{Flag, PolicyNetwork};

pub fn gradient_batch(
    threads: usize,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    batch: &[TrainingPosition],
) -> f32 {
    let size = (batch.len() / threads).max(1);
    let mut errors = vec![0.0; threads];

    std::thread::scope(|s| {
        batch
            .chunks(size)
            .zip(errors.iter_mut())
            .map(|(chunk, err)| {
                s.spawn(move || {
                    let mut inner_grad = PolicyNetwork::boxed_and_zeroed();
                    for pos in chunk {
                        update_single_grad(pos, policy, &mut inner_grad, err);
                    }
                    inner_grad
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|p| p.join().unwrap())
            .for_each(|part| *grad += &part);
    });

    errors.iter().sum::<f32>()
}

fn update_single_grad(
    pos: &TrainingPosition,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    error: &mut f32,
) {
    let feats = pos.board().get_features();

    let mut policies = Vec::with_capacity(pos.num_moves());
    let mut total = 0.0;
    let mut total_visits = 0;
    let mut max = -1000.0;

    let flip = pos.board().flip_val();

    for training_mov in pos.moves() {
        let mov = training_mov.mov(pos.board());
        let visits = training_mov.visits();
        let from = usize::from(mov.from() ^ flip);
        let to = 64 + usize::from(mov.to() ^ flip);

        let from_out = policy.weights[from].out_with_layers(&feats);
        let to_out = policy.weights[to].out_with_layers(&feats);

        let net_out = from_out.dot(&to_out);

        let score = net_out + policy.hce(&mov, pos.board());

        if score > max {
            max = score;
        }

        total_visits += visits;
        policies.push((mov, visits, score, from_out, to_out));
    }

    for (_, _, score, _, _) in policies.iter_mut() {
        *score = (*score - max).exp();
        total += *score;
    }

    for (mov, visits, score, from_out, to_out) in policies {
        let from = usize::from(mov.from() ^ flip);
        let to = 64 + usize::from(mov.to() ^ flip);

        let ratio = score / total;

        let expected = visits as f32 / total_visits as f32;
        let err = ratio - expected;

        *error += err * err;

        let factor = err * ratio * (1.0 - ratio);

        policy.weights[from].backprop(
            &feats,
            factor,
            &mut grad.weights[from],
            to_out,
            from_out,
        );

        policy.weights[to].backprop(
            &feats,
            factor,
            &mut grad.weights[to],
            from_out,
            to_out,
        );

        if pos.board().see(&mov, -108) {
            grad.hce[0] += factor;
        }

        if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
            grad.hce[1] += factor;
        }

        if mov.is_capture() {
            grad.hce[2] += factor;

            let diff = pos.board().get_pc(1 << mov.to()) as i32 - i32::from(mov.moved());
            grad.hce[3] += factor * diff as f32;
        }
    }
}
