use datagen::{impls::ataxx::AtaxxPolicyData, to_slice_with_lifetime, Rand};
use goober::{FeedForwardNetwork, OutputLayer};
use monty::ataxx::{PolicyNetwork, SubNet};

use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

const BATCH_SIZE: usize = 16_384;
const EPOCHS: usize = 4;
const LR_DROP: usize = 3;

pub fn train_policy(threads: usize, data_path: &str) {
    let file = File::open(data_path).unwrap();

    let mut policy = PolicyNetwork::boxed_and_zeroed();
    let mut rng = Rand::with_seed();
    for subnet in policy.subnets.iter_mut() {
        *subnet = SubNet::from_fn(|| rng.rand_f32(0.2));
    }

    println!("# [Info]");
    println!(
        "> {} Positions",
        file.metadata().unwrap().len() / std::mem::size_of::<AtaxxPolicyData>() as u64,
    );

    let mut lr = 0.001;
    let mut momentum = PolicyNetwork::boxed_and_zeroed();
    let mut velocity = PolicyNetwork::boxed_and_zeroed();

    for iteration in 1..=EPOCHS {
        println!("# [Training Epoch {iteration}]");
        train(
            threads,
            &mut policy,
            lr,
            &mut momentum,
            &mut velocity,
            data_path,
        );

        if iteration % LR_DROP == 0 {
            lr *= 0.1;
        }

        policy.write_to_bin(format!("checkpoints/ataxx-policy-{iteration}.bin").as_str());
    }
}

fn train(
    threads: usize,
    policy: &mut PolicyNetwork,
    lr: f32,
    momentum: &mut PolicyNetwork,
    velocity: &mut PolicyNetwork,
    path: &str,
) {
    let mut running_error = 0.0;
    let mut num = 0;

    let cap = 128 * BATCH_SIZE * std::mem::size_of::<AtaxxPolicyData>();
    let file = File::open(path).unwrap();
    let size = file.metadata().unwrap().len() as usize / std::mem::size_of::<AtaxxPolicyData>();
    let mut loaded = BufReader::with_capacity(cap, file);
    let mut batch_no = 0;
    let num_batches = (size + BATCH_SIZE - 1) / BATCH_SIZE;

    while let Ok(buf) = loaded.fill_buf() {
        if buf.is_empty() {
            break;
        }

        let data = to_slice_with_lifetime(buf);

        for batch in data.chunks(BATCH_SIZE) {
            let mut grad = PolicyNetwork::boxed_and_zeroed();
            running_error += gradient_batch(threads, policy, &mut grad, batch);
            let adj = 2.0 / batch.len() as f32;
            update(policy, &grad, adj, lr, momentum, velocity);

            batch_no += 1;
            print!("> Batch {batch_no}/{num_batches}\r");
            let _ = std::io::stdout().flush();
        }

        num += data.len();
        let consumed = buf.len();
        loaded.consume(consumed);
    }

    println!("> Running Loss: {}", running_error / num as f32);
}

fn update(
    policy: &mut PolicyNetwork,
    grad: &PolicyNetwork,
    adj: f32,
    lr: f32,
    momentum: &mut PolicyNetwork,
    velocity: &mut PolicyNetwork,
) {
    for (i, subnet) in policy.subnets.iter_mut().enumerate() {
        subnet.adam(
            &grad.subnets[i],
            &mut momentum.subnets[i],
            &mut velocity.subnets[i],
            adj,
            lr,
        );
    }
}

fn gradient_batch(
    threads: usize,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    batch: &[AtaxxPolicyData],
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
    pos: &AtaxxPolicyData,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    error: &mut f32,
) {
    if pos.num == 1 && pos.moves[0].from == 63 {
        return;
    }

    let board = pos.board;

    let feats = board.get_features();

    let mut policies = Vec::with_capacity(pos.num);
    let mut total = 0.0;
    let mut total_visits = 0;
    let mut max = -1000.0;

    for mov in &pos.moves[..pos.num] {
        let visits = mov.visits;
        let from = usize::from(mov.from.min(49));
        let to = 50 + usize::from(mov.to.min(48));

        let from_out = policy.subnets[from].out_with_layers(&feats);
        let to_out = policy.subnets[to].out_with_layers(&feats);

        let score = from_out.output_layer().dot(&to_out.output_layer());

        if score > max {
            max = score;
        }

        total_visits += visits;
        policies.push((mov, score, from_out, to_out));
    }

    for (_, score, _, _) in policies.iter_mut() {
        *score = (*score - max).exp();
        total += *score;
    }

    for (mov, score, from_out, to_out) in policies {
        let visits = mov.visits;
        let from = usize::from(mov.from.min(49));
        let to = 50 + usize::from(mov.to.min(48));

        let ratio = score / total;

        let expected = visits as f32 / total_visits as f32;
        let err = ratio - expected;

        *error -= expected * ratio.ln();

        let factor = err;

        policy.subnets[from].backprop(
            &feats,
            &mut grad.subnets[from],
            factor * to_out.output_layer(),
            &from_out,
        );

        policy.subnets[to].backprop(
            &feats,
            &mut grad.subnets[to],
            factor * from_out.output_layer(),
            &to_out,
        );
    }
}