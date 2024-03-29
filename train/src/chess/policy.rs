use datagen::{impls::chess::ChessPolicyData, to_slice_with_lifetime, Rand};
use goober::{FeedForwardNetwork, OutputLayer};
use monty::chess::{consts::Flag, PolicyNetwork, SubNet};

use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

const BATCH_SIZE: usize = 16_384;
const EPOCHS: usize = 10;
const LR_DROP: usize = 7;

pub fn train_policy() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let data_path = args.next().unwrap();

    let file = File::open(data_path.clone()).unwrap();

    let mut policy = PolicyNetwork::boxed_and_zeroed();
    let mut rng = Rand::with_seed();
    for subnet in policy.weights.iter_mut() {
        *subnet = SubNet::from_fn(|| rng.rand_f32(0.2));
    }

    println!("# [Info]");
    println!(
        "> {} Positions",
        file.metadata().unwrap().len() / std::mem::size_of::<ChessPolicyData>() as u64,
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
            data_path.as_str(),
        );

        if iteration % LR_DROP == 0 {
            lr *= 0.1;
        }
        println!("{:?}", policy.hce);
        policy.write_to_bin(format!("resources/chess-policy-{iteration}.bin").as_str());
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

    let cap = 128 * BATCH_SIZE * std::mem::size_of::<ChessPolicyData>();
    let file = File::open(path).unwrap();
    let size = file.metadata().unwrap().len() as usize / std::mem::size_of::<ChessPolicyData>();
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

const B1: f32 = 0.9;
const B2: f32 = 0.999;

fn update(
    policy: &mut PolicyNetwork,
    grad: &PolicyNetwork,
    adj: f32,
    lr: f32,
    momentum: &mut PolicyNetwork,
    velocity: &mut PolicyNetwork,
) {
    for (i, subnet) in policy.weights.iter_mut().enumerate() {
        subnet.adam(
            &grad.weights[i],
            &mut momentum.weights[i],
            &mut velocity.weights[i],
            adj,
            lr,
        );
    }

    for (i, p) in policy.hce.iter_mut().enumerate() {
        let g = adj * grad.hce[i];
        let m = &mut momentum.hce[i];
        let v = &mut velocity.hce[i];

        *m = B1 * *m + (1. - B1) * g;
        *v = B2 * *v + (1. - B2) * g * g;
        *p -= lr * *m / (v.sqrt() + 0.000_000_01);
    }
}

fn gradient_batch(
    threads: usize,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    batch: &[ChessPolicyData],
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
    pos: &ChessPolicyData,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    error: &mut f32,
) {
    let board = pos.board;

    let feats = board.get_features();

    let mut policies = Vec::with_capacity(pos.num);
    let mut total = 0.0;
    let mut total_visits = 0;
    let mut max = -1000.0;

    let flip = board.flip_val();

    for training_mov in &pos.moves[..pos.num] {
        let mov = board.move_from_u16(training_mov.mov);

        let visits = training_mov.visits;
        let from = usize::from(mov.from() ^ flip);
        let to = 64 + usize::from(mov.to() ^ flip);

        let from_out = policy.weights[from].out_with_layers(&feats);
        let to_out = policy.weights[to].out_with_layers(&feats);

        let net_out = from_out.output_layer().dot(&to_out.output_layer());

        let score = net_out + policy.hce(&mov, &board);

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

        *error -= expected * ratio.ln();

        let factor = err;

        policy.weights[from].backprop(
            &feats,
            &mut grad.weights[from],
            factor * to_out.output_layer(),
            &from_out,
        );

        policy.weights[to].backprop(
            &feats,
            &mut grad.weights[to],
            factor * from_out.output_layer(),
            &to_out,
        );

        if board.see(&mov, -108) {
            grad.hce[0] += factor;
        }

        if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
            grad.hce[1] += factor;
        }

        if mov.is_capture() {
            grad.hce[2] += factor;

            let diff = board.get_pc(1 << mov.to()) as i32 - i32::from(mov.moved());
            grad.hce[3] += factor * diff as f32;
        }
    }
}
