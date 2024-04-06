use bullet::{
    inputs, outputs, Activation, LocalSettings, Loss, LrScheduler, TrainerBuilder,
    TrainingSchedule, WdlScheduler,
};

const HIDDEN_SIZE: usize = 16;

fn main() {
    let mut trainer = TrainerBuilder::default()
        .single_perspective()
        .quantisations(&[255, 64])
        .input(inputs::Chess768)
        .output_buckets(outputs::Single)
        .feature_transformer(HIDDEN_SIZE)
        .activate(Activation::SCReLU)
        .add_layer(1)
        .build();

    let schedule = TrainingSchedule {
        net_id: "chess-value001".to_string(),
        eval_scale: 400.0,
        ft_regularisation: 0.0,
        batch_size: 16_384,
        batches_per_superbatch: 512,
        start_superbatch: 1,
        end_superbatch: 40,
        wdl_scheduler: WdlScheduler::Constant { value: 0.5 },
        lr_scheduler: LrScheduler::Step {
            start: 0.001,
            gamma: 0.1,
            step: 15,
        },
        loss_function: Loss::SigmoidMSE,
        save_rate: 10,
    };

    let settings = LocalSettings {
        threads: 4,
        data_file_paths: vec!["data/chess/value001.data"],
        output_directory: "checkpoints",
    };

    trainer.run(&schedule, &settings);
}
