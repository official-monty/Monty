mod dataloader;
mod input;

use dataloader::MontyBinpackLoader;
use input::ThreatInputs;

use bullet_lib::{
    game::inputs::SparseInputType,
    nn::{
        optimiser::{AdamW, AdamWParams},
        InitSettings, Shape,
    },
    trainer::{
        save::SavedFormat,
        schedule::{lr::ExponentialDecayLR, wdl::ConstantWDL, TrainingSchedule, TrainingSteps},
        settings::LocalSettings,
    },
    value::ValueTrainerBuilder,
};

use montyformat::chess::{Move, Position};

fn main() {
    let experiment_name = "3072T".to_string();

    // architecture
    let input_features = ThreatInputs;
    let l1 = 3072;
    let l2 = 16;
    let l3 = 128;

    // training schedule
    let initial_lr = 0.001;
    let final_lr = 0.0000001;
    let superbatches = 4000;

    // data
    let data_path = "/home/privateclient/monty_value_training/interleaved-value.binpack";
    let dataloader_buffer_size_mb = 96000;
    let dataloader_threads = 8;

    let mut trainer = ValueTrainerBuilder::default()
        .wdl_output()
        .inputs(input_features)
        .optimiser(AdamW)
        .save_format(&[
            SavedFormat::id("pst"),
            SavedFormat::id("l0w").quantise::<i8>(128).round(),
            SavedFormat::id("l0b").quantise::<i8>(128).round(),
            SavedFormat::id("l1w")
                .quantise::<i16>(1024)
                .transpose()
                .round(),
            SavedFormat::id("l1b").quantise::<i16>(1024).round(),
            SavedFormat::id("l2w"),
            SavedFormat::id("l2b"),
            SavedFormat::id("l3w"),
            SavedFormat::id("l3b"),
        ])
        .build_custom(|builder, inputs, targets| {
            let num_inputs = input_features.num_inputs();

            let pst = builder.new_weights("pst", Shape::new(3, num_inputs), InitSettings::Zeroed);
            let l0 = builder.new_affine("l0", num_inputs, l1);
            let l1 = builder.new_affine("l1", l1 / 2, l2);
            let l2 = builder.new_affine("l2", l2, l3);
            let l3 = builder.new_affine("l3", l3, 3);

            l0.init_with_effective_input_size(input_features.max_active());

            let l0 = l0.forward(inputs).crelu().pairwise_mul();
            let l1 = l1.forward(l0).screlu();
            let l2 = l2.forward(l1).screlu();
            let l3 = l3.forward(l2);
            let out = l3 + pst.matmul(inputs);

            let ones = builder.new_constant(Shape::new(1, 3), &[1.0; 3]);
            let loss = ones.matmul(out.softmax_crossentropy_loss(targets));

            (out, loss)
        });

    let optimiser_params = AdamWParams {
        decay: 0.01,
        beta1: 0.9,
        beta2: 0.999,
        min_weight: -0.99,
        max_weight: 0.99,
    };

    trainer.optimiser.set_params(optimiser_params);

    let schedule = TrainingSchedule {
        net_id: experiment_name,
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 65_536,
            batches_per_superbatch: 1526,
            start_superbatch: 1,
            end_superbatch: superbatches,
        },
        wdl_scheduler: ConstantWDL { value: 1.0 },
        lr_scheduler: ExponentialDecayLR {
            initial_lr,
            final_lr,
            final_superbatch: superbatches,
        },
        save_rate: 200,
    };

    let settings = LocalSettings {
        threads: 2,
        test_set: None,
        output_directory: "checkpoints",
        batch_queue_size: 32,
    };

    fn filter(_: &Position, _: Move, _: i16, _: f32) -> bool {
        true
    }

    let data_loader = MontyBinpackLoader::new(
        data_path,
        dataloader_buffer_size_mb,
        dataloader_threads,
        filter,
    );

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
