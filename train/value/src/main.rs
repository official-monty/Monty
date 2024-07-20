use bullet::{
    format::{chess::BoardIter, ChessBoard},
    inputs, outputs, Activation, LocalSettings, Loss, LrScheduler, TrainerBuilder,
    TrainingSchedule, WdlScheduler,
};
use monty::Board;

const HIDDEN_SIZE: usize = 512;

fn main() {
    let mut trainer = TrainerBuilder::default()
        .single_perspective()
        .input(ThreatInputs)
        .output_buckets(outputs::Single)
        .feature_transformer(HIDDEN_SIZE)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(16)
        .activate(Activation::SCReLU)
        .add_layer(1)
        .build();

    let schedule = TrainingSchedule {
        net_id: "datagen0-5".to_string(),
        eval_scale: 400.0,
        ft_regularisation: 0.0,
        batch_size: 16_384,
        batches_per_superbatch: 6104,
        start_superbatch: 1,
        end_superbatch: 160,
        wdl_scheduler: WdlScheduler::Constant { value: 0.5 },
        lr_scheduler: LrScheduler::Step {
            start: 0.001,
            gamma: 0.1,
            step: 60,
        },
        loss_function: Loss::SigmoidMSE,
        save_rate: 10,
    };

    let settings = LocalSettings {
        threads: 4,
        data_file_paths: vec!["../monty-data/datagen0-5.data"],
        output_directory: "checkpoints",
    };

    trainer.run(&schedule, &settings);

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

#[derive(Clone, Copy, Default)]
pub struct ThreatInputs;

pub struct ThreatInputsIter {
    board_iter: BoardIter,
    threats: u64,
    defences: u64,
    flip: u8,
}

impl inputs::InputType for ThreatInputs {
    type RequiredDataType = ChessBoard;
    type FeatureIter = ThreatInputsIter;

    fn buckets(&self) -> usize {
        1
    }

    fn max_active_inputs(&self) -> usize {
        32
    }

    fn inputs(&self) -> usize {
        768 * 4
    }

    fn feature_iter(&self, pos: &Self::RequiredDataType) -> Self::FeatureIter {
        let mut bb = [0; 8];

        for (pc, sq) in pos.into_iter() {
            let bit = 1 << sq;
            bb[usize::from(pc >> 3)] ^= bit;
            bb[usize::from(2 + (pc & 7))] ^= bit;
        }

        let board = Board::from_raw(bb, false, 0, 0, 0);

        let threats = board.threats_by(1);
        let defences = board.threats_by(0);

        ThreatInputsIter {
            board_iter: pos.into_iter(),
            threats,
            defences,
            flip: if pos.our_ksq() % 8 > 3 { 7 } else { 0 },
        }
    }
}

impl Iterator for ThreatInputsIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.board_iter.next().map(|(piece, square)| {
            let c = usize::from(piece & 8 > 0);
            let pc = 64 * usize::from(piece & 7);
            let sq = usize::from(square);
            let mut feat = [0, 384][c] + pc + (sq ^ usize::from(self.flip));

            if self.threats & (1 << sq) > 0 {
                feat += 768;
            }

            if self.defences & (1 << sq) > 0 {
                feat += 768 * 2;
            }

            (feat, feat)
        })
    }
}
