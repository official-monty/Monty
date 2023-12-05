use monty_core::{PolicyNetwork, NetworkDims, PolicyVal};
use monty_train::{gradient_batch, TrainingPosition, to_slice_with_lifetime, Rand};

use std::{fs::File, io::{BufReader, BufRead, Write}};

const BATCH_SIZE: usize = 16_384;
const EPOCHS: usize = 10;
const LR_DROP: usize = 7;

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let data_path = args.next().unwrap();

    let file = File::open(data_path.clone()).unwrap();

    let mut policy = PolicyNetwork::boxed_and_zeroed();
    let mut rng = Rand::with_seed();
    for i in 0..NetworkDims::INDICES {
        for j in 0..NetworkDims::FEATURES {
            let mut val = [0.0; NetworkDims::NEURONS];
            for v in val.iter_mut() {
                *v = rng.rand_f32(0.2);
            }
            policy.weights[i][j] = PolicyVal::from_raw(val);
        }
    }

    let mut val = [0.0; NetworkDims::NEURONS];
    for v in val.iter_mut() {
        *v = rng.rand_f32(0.1);
    }
    policy.outputs = PolicyVal::from_raw(val);

    println!("# [Info]");
    println!(
        "> {} Positions",
        file.metadata().unwrap().len() / std::mem::size_of::<TrainingPosition>() as u64,
    );

    let mut lr = 0.001;
    let mut momentum = PolicyNetwork::boxed_and_zeroed();
    let mut velocity = PolicyNetwork::boxed_and_zeroed();

    for iteration in 1..=EPOCHS {
        println!("# [Training Epoch {iteration}]");
        train(threads, &mut policy, lr, &mut momentum, &mut velocity, data_path.as_str());

        if iteration % LR_DROP == 0 {
            lr *= 0.1;
        }
        println!("{:?}", policy.hce);
        policy.write_to_bin(format!("resources/policy-{iteration}.bin").as_str());
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

    let cap = 128 * BATCH_SIZE * std::mem::size_of::<TrainingPosition>();
    let file = File::open(path).unwrap();
    let size = file.metadata().unwrap().len() as usize / std::mem::size_of::<TrainingPosition>();
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
    for i in 0..NetworkDims::INDICES {
        for j in 0..NetworkDims::FEATURES {
            let g = adj * grad.weights[i][j];
            let m = &mut momentum.weights[i][j];
            let v = &mut velocity.weights[i][j];
            let p = &mut policy.weights[i][j];

            *m = B1 * *m + (1. - B1) * g;
            *v = B2 * *v + (1. - B2) * g * g;
            *p -= lr * *m / (v.sqrt() + 0.000_000_01);
        }
    }

    let g = adj * grad.outputs;
    let m = &mut momentum.outputs;
    let v = &mut velocity.outputs;
    let p = &mut policy.outputs;
    *m = B1 * *m + (1. - B1) * g;
    *v = B2 * *v + (1. - B2) * g * g;
    *p -= lr * *m / (v.sqrt() + 0.000_000_01);

    for i in 0..NetworkDims::HCE {
        let g = adj * grad.hce[i];
        let m = &mut momentum.hce[i];
        let v = &mut velocity.hce[i];
        let p = &mut policy.hce[i];

        *m = B1 * *m + (1. - B1) * g;
        *v = B2 * *v + (1. - B2) * g * g;
        *p -= lr * *m / (v.sqrt() + 0.000_000_01);
    }
}
