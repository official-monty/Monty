mod datagen;
mod rng;

use crate::{search::{policy::{PolicyNetwork, hce_policy}, params::TunableParams}, state::{consts::Piece, position::Position}, pop_lsb};
use self::{datagen::{run_datagen, TrainingPosition}, rng::Rand};

const DATAGEN_SIZE: usize = 16_384;
const BATCH_SIZE: usize = 1_024;
const LR: f64 = 1.0;

pub fn run_training(threads: usize, params: TunableParams, _policy: &mut PolicyNetwork) {
    let mut policy = PolicyNetwork::boxed_and_zeroed();

    for iteration in 1..=64 {
        println!("# [Generating Data]");
        let mut data = run_datagen(threads, DATAGEN_SIZE, params.clone(), &policy);

        println!("# [Shuffling]");
        shuffle(&mut data);

        println!("# [Training]");
        train(threads, &mut policy, data);

        policy.write_to_bin(format!("policy-{iteration}.bin").as_str());
    }
}

fn shuffle(data: &mut Vec<TrainingPosition>) {
    let mut rng = Rand::with_seed();

    for _ in 0..data.len() * 4 {
        let idx1 = rng.rand_int() as usize % data.len();
        let idx2 = rng.rand_int() as usize % data.len();
        data.swap(idx1, idx2);
    }
}

fn train(threads: usize, policy: &mut PolicyNetwork, data: Vec<TrainingPosition>) {
    let mut grad = PolicyNetwork::boxed_and_zeroed();
    let error = gradient_batch(threads, policy, &mut grad, &data);
    println!("> Before Loss: {}", error / data.len() as f64);

    let mut running_error = 0.0;

    for batch in data.chunks(BATCH_SIZE) {
        let mut grad = PolicyNetwork::boxed_and_zeroed();
        running_error += gradient_batch(threads, policy, &mut grad, batch);
        let adj = LR / batch.len() as f64;
        update(policy, &grad, adj);
    }

    println!("> Running Loss: {}", running_error / data.len() as f64);

    let mut grad = PolicyNetwork::boxed_and_zeroed();
    let error = gradient_batch(threads, policy, &mut grad, &data);
    println!("> After Loss: {}", error / data.len() as f64);
}

fn gradient_batch(threads: usize, policy: &PolicyNetwork, grad: &mut PolicyNetwork, batch: &[TrainingPosition]) -> f64 {
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

    errors.iter().sum::<f64>()
}

fn get_features(pos: &Position) -> Vec<usize> {
    let mut res = Vec::with_capacity(pos.occ().count_ones() as usize + 1);
    let flip = pos.flip_val();

    // bias is just an always-present feature
    res.push(768);

    for piece in Piece::PAWN..=Piece::KING {
        let pc = 64 * (piece - 2);

        let mut our_bb = pos.piece(piece) & pos.piece(pos.stm());
        while our_bb > 0 {
            pop_lsb!(sq, our_bb);
            res.push(pc + usize::from(sq ^ flip));
        }

        let mut opp_bb = pos.piece(piece) & pos.piece(pos.stm() ^ 1);
        while opp_bb > 0 {
            pop_lsb!(sq, opp_bb);
            res.push(384 + pc + usize::from(sq ^ flip));
        }
    }

    res
}

fn update_single_grad(pos: &TrainingPosition, policy: &PolicyNetwork, grad: &mut PolicyNetwork, error: &mut f64) {
    let feats = get_features(&pos.position);

    let mut policies = Vec::with_capacity(pos.moves.len());
    let mut total = 0.0;
    let mut total_visits = 0;

    let flip = pos.position.flip_val();

    for (mov, visits) in &pos.moves {
        let idx = mov.index(flip);

        let mut score = hce_policy(mov, &pos.position);
        for &feat in &feats {
            score += policy.weights[idx][feat];
        }

        score = score.exp();

        total += score;
        total_visits += visits;
        policies.push(score);
    }

    for ((mov, visits), score) in pos.moves.iter().zip(policies.iter()) {
        let idx = mov.index(flip);
        let expected = f64::from(*visits) / f64::from(total_visits);
        let err = score / total - expected;

        *error += err * err;

        let dp = (total - score) / total.powi(2);
        let adj = 2.0 * err * score * dp;

        for &feat in &feats {
            grad.weights[idx][feat] += adj;
        }
    }
}

fn update(policy: &mut PolicyNetwork, grad: &PolicyNetwork, adj: f64) {
    for (i, j) in policy.weights.iter_mut().zip(grad.weights.iter()) {
        for (a, b) in i.iter_mut().zip(j.iter()) {
            *a -= adj * *b;
        }
    }
}
