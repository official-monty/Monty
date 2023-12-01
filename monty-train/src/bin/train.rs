use monty_engine::{PolicyNetwork, NetworkDims};
use monty_train::{data_from_bytes_with_lifetime, gradient_batch, TrainingPosition};

const BATCH_SIZE: usize = 16_384;

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let data_path = args.next().unwrap();

    let mut raw_bytes = std::fs::read(data_path).unwrap();
    let data = data_from_bytes_with_lifetime(&mut raw_bytes);

    let mut policy = PolicyNetwork::boxed_and_zeroed();

    println!("# [Info]");
    println!("> {} Positions", data.len());

    let mut lr = 0.001;
    let mut momentum = PolicyNetwork::boxed_and_zeroed();
    let mut velocity = PolicyNetwork::boxed_and_zeroed();

    for iteration in 1..=20 {
        println!("# [Training Epoch {iteration}]");
        train(threads, &mut policy, data, lr, &mut momentum, &mut velocity);

        if iteration % 5 == 0 {
            lr *= 0.1;
        }
        policy.write_to_bin("policy.bin");
    }
}

fn train(
    threads: usize,
    policy: &mut PolicyNetwork,
    data: &[TrainingPosition],
    lr: f32,
    momentum: &mut PolicyNetwork,
    velocity: &mut PolicyNetwork,
) {
    let mut running_error = 0.0;

    for batch in data.chunks(BATCH_SIZE) {
        let mut grad = PolicyNetwork::boxed_and_zeroed();
        running_error += gradient_batch(threads, policy, &mut grad, batch);
        let adj = 2.0 / batch.len() as f32;
        update(policy, &grad, adj, lr, momentum, velocity);
    }

    println!("> Running Loss: {}", running_error / data.len() as f32);
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
            assert!(!p.is_nan() && !p.is_infinite(), "{}, {}, {}, {}", *p, g, *m, *v);
        }
    }
}