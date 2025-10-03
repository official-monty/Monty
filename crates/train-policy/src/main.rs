pub mod data;
pub mod model;

use acyclib::{
    device::Device,
    trainer::{
        optimiser::{
            adam::{AdamW, AdamWParams},
            Optimiser,
        },
        schedule::{TrainingSchedule, TrainingSteps},
        Trainer,
    },
};
use bullet_cuda_backend::CudaDevice;

use data::MontyDataLoader;

fn main() {
    let hl = 16384;
    let dataloader = MontyDataLoader::new(
        "/home/privateclient/monty_value_training/interleaved.binpack",
        96000,
        4,
        8,
    );

    let device = CudaDevice::new(0).unwrap();

    let (graph, node) = model::make(device, hl);

    let params = AdamWParams {
        decay: 0.01,
        beta1: 0.9,
        beta2: 0.999,
        min_weight: -0.99,
        max_weight: 0.99,
    };
    let optimiser = Optimiser::<_, _, AdamW<_>>::new(graph, params).unwrap();

    let mut trainer = Trainer {
        optimiser,
        state: (),
    };

    let save_rate = 40;
    let end_superbatch = 800;
    let initial_lr = 0.001;
    let final_lr = 0.00001;

    let steps = TrainingSteps {
        batch_size: 16384,
        batches_per_superbatch: 6104,
        start_superbatch: 1,
        end_superbatch,
    };

    let schedule = TrainingSchedule {
        steps,
        log_rate: 64,
        lr_schedule: Box::new(|_, sb| {
            if sb >= end_superbatch {
                return final_lr;
            }

            let lambda = sb as f32 / end_superbatch as f32;
            initial_lr * (final_lr / initial_lr).powf(lambda)
        }),
    };

    trainer
        .train_custom(
            schedule,
            dataloader,
            |_, _, _, _| {},
            |trainer, superbatch| {
                if superbatch % save_rate == 0 || superbatch == steps.end_superbatch {
                    println!("Saving Checkpoint");
                    let dir = format!("checkpoints/policy-{superbatch}");
                    let _ = std::fs::create_dir(&dir);
                    trainer.optimiser.write_to_checkpoint(&dir).unwrap();
                    model::save_quantised(
                        &trainer.optimiser.graph,
                        &format!("{dir}/quantised.bin"),
                    )
                    .unwrap();
                }
            },
        )
        .unwrap();

    model::eval(
        &mut trainer.optimiser.graph,
        node,
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    );
}
