mod input;
mod loader;

use bullet::{
    lr, optimiser, outputs, wdl, Activation, LocalSettings, Loss, TrainerBuilder, TrainingSchedule,
    TrainingSteps,
};

const HIDDEN_SIZE: usize = 4096;

fn main() {
    let mut trainer = TrainerBuilder::default()
        .optimiser(optimiser::AdamW)
        .loss_fn(Loss::SigmoidMSE)
        .single_perspective()
        .input(input::ThreatInputs)
        .output_buckets(outputs::Single)
        .feature_transformer(HIDDEN_SIZE)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(128)
        .activate(Activation::SCReLU)
        .add_layer(1)
        .build();

    let schedule = TrainingSchedule {
        net_id: "4096EXP".to_string(),
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: 6104,
            start_superbatch: 1,
            end_superbatch: 3000,
        },
        wdl_scheduler: wdl::ConstantWDL { value: 1.0 },
        lr_scheduler: lr::ExponentialDecayLR {
            initial_lr: 0.001,
            final_lr: 0.0000001,
            final_superbatch: 3000,
        },
        save_rate: 20,
    };

    let optimiser_params = optimiser::AdamWParams {
        decay: 0.01,
        beta1: 0.9,
        beta2: 0.999,
        min_weight: -0.99,
        max_weight: 0.99,
    };

    trainer.set_optimiser_params(optimiser_params);

    let settings = LocalSettings {
        threads: 8,
        test_set: None,
        output_directory: "checkpoints",
        batch_queue_size: 256,
    };

    let data_loader = loader::BinpackLoader::new("../binpacks/bestmove-q.binpack", 48000);

    trainer.run(&schedule, &settings, &data_loader);

    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    ] {
        let eval = trainer.eval(fen);
        println!("FEN: {fen}");
        println!("EVAL: {}", 400.0 * eval);
    }
}
