mod arch;
mod loader;

use arch::PolicyNetwork;
use datagen::PolicyData;
use loader::DataLoader;

use std::{io::Write, time::Instant};

const BATCH_SIZE: usize = 16_384;
const BPSB: usize = 1024;

pub fn train(
    buffer_size_mb: usize,
    threads: usize,
    data_path: String,
    superbatches: usize,
    lr_drop: usize,
) {
    let mut policy = PolicyNetwork::rand_init();

    let mut lr = 0.001;
    let mut momentum = PolicyNetwork::boxed_and_zeroed();
    let mut velocity = PolicyNetwork::boxed_and_zeroed();

    let mut running_error = 0.0;
    let mut sb = 0;
    let mut batch_no = 0;

    let data_loader = DataLoader::new(data_path.as_str(), buffer_size_mb, BATCH_SIZE);

    let mut t = Instant::now();

    data_loader.map_batches(|batch| {
        let mut grad = PolicyNetwork::boxed_and_zeroed();
        running_error += gradient_batch(threads, &policy, &mut grad, batch);
        let adj = 1.0 / batch.len() as f32;
        PolicyNetwork::update(&mut policy, &grad, adj, lr, &mut momentum, &mut velocity);

        batch_no += 1;
        print!(
            "> Superbatch {}/{superbatches} Batch {}/{BPSB}\r",
            sb + 1,
            batch_no % BPSB,
        );
        let _ = std::io::stdout().flush();

        if batch_no % BPSB == 0 {
            let elapsed = t.elapsed().as_secs_f32();
            t = Instant::now();

            sb += 1;
            println!(
                "> Superbatch {sb}/{superbatches} Running Loss {} Time {:.2}s",
                running_error / (BPSB * BATCH_SIZE) as f32,
                elapsed,
            );

            let mut seconds_left = ((superbatches - sb) as f32 * elapsed) as u64;
            let mut minutes_left = seconds_left / 60;
            seconds_left -= minutes_left * 60;
            let hours_left = minutes_left / 60;
            minutes_left -= hours_left * 60;

            println!("Estimated {hours_left}h {minutes_left}m {seconds_left}s Left in Training",);

            running_error = 0.0;

            if sb % lr_drop == 0 {
                lr *= 0.1;
                println!("Dropping LR to {lr}");
            }

            policy.write_to_bin(format!("checkpoints/policy-{sb}.bin").as_str());

            sb == superbatches
        } else {
            false
        }
    });
}

fn gradient_batch(
    threads: usize,
    policy: &PolicyNetwork,
    grad: &mut PolicyNetwork,
    batch: &[PolicyData],
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
                        PolicyNetwork::update_single_grad(pos, policy, &mut inner_grad, err);
                    }
                    inner_grad
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|p| p.join().unwrap())
            .for_each(|part| grad.add_without_explicit_lifetime(&part));
    });

    errors.iter().sum::<f32>()
}
