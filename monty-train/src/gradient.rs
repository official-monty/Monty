use crate::TrainingPosition;

use monty_engine::PolicyNetwork;

pub fn gradient_batch(threads: usize, policy: &PolicyNetwork, grad: &mut PolicyNetwork, batch: &[TrainingPosition]) -> f32 {
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

fn update_single_grad(pos: &TrainingPosition, policy: &PolicyNetwork, grad: &mut PolicyNetwork, error: &mut f32) {
    let feats = pos.board().get_features();

    let mut policies = Vec::with_capacity(pos.num_moves());
    let mut total = 0.0;
    let mut total_visits = 0;
    let mut max = -1000.0;

    let flip = pos.board().flip_val();

    for training_mov in pos.moves() {
        let mov = training_mov.mov(pos.board());
        let visits = training_mov.visits();

        let mut score = PolicyNetwork::hce(&mov, pos.board());
        let idx = mov.index(flip);

        for &feat in feats.iter() {
            score += policy.weights[idx][feat];
        }

        if score > max {
            max = score;
        }

        total_visits += visits;
        policies.push((mov, visits, score));
    }

    for (_, _, score) in policies.iter_mut() {
        *score = (*score - max).exp();
        total += *score;
    }

    for (mov, visits, score) in policies {
        let idx = mov.index(flip);

        let ratio = score / total;

        let expected = visits as f32 / total_visits as f32;
        let err = ratio - expected;

        *error += err * err;

        let adj = err * ratio * (1.0 - ratio);

        for &feat in feats.iter() {
            grad.weights[idx][feat] += adj;
        }
    }
}