use crate::{to_slice_with_lifetime, Rand, TrainingPosition};

use monty_core::{GameState, PolicyNetwork, Position, STARTPOS};
use monty_engine::{Searcher, TunableParams};

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::atomic::{AtomicBool, Ordering}, time::Instant,
};

const NODES_PER_MOVE: usize = 1_000;

static STOP: AtomicBool = AtomicBool::new(false);

fn stop_is_set() -> bool {
    STOP.load(Ordering::Relaxed)
}

pub fn set_stop() {
    STOP.store(true, Ordering::Relaxed);
}

pub fn write_data(data: &[TrainingPosition], output: &mut BufWriter<File>) {
    if data.is_empty() {
        return;
    }

    let data_slice = to_slice_with_lifetime(data);

    output
        .write_all(data_slice)
        .expect("Nothing can go wrong in unsafe code!");
}

pub struct DatagenThread<'a> {
    id: u32,
    rng: Rand,
    params: TunableParams,
    policy: &'a PolicyNetwork,
    positions: Vec<TrainingPosition>,
    skipped: usize,
    total: usize,
    timer: Instant,
}

impl<'a> DatagenThread<'a> {
    pub fn new(id: u32, params: TunableParams, policy: &'a PolicyNetwork) -> Self {
        Self {
            id,
            rng: Rand::with_seed(),
            params,
            policy,
            positions: Vec::new(),
            skipped: 0,
            total: 0,
            timer: Instant::now(),
        }
    }

    pub fn run(&mut self) {
        let position = Position::parse_fen(STARTPOS);

        let out_path = format!("monty-{}.data", self.rng.rand_int());
        let mut output =
            BufWriter::new(File::create(out_path.as_str()).expect("Provide a correct path!"));

        loop {
            if stop_is_set() {
                break;
            }

            self.run_game(position, self.params.clone(), self.policy);

            let num_in_buffer = self.positions.len();
            if num_in_buffer > 2048 {
                self.write(&mut output);
            }
        }

        if !self.positions.is_empty() {
            self.write(&mut output);
        }
    }

    fn write(&mut self, output: &mut BufWriter<File>) {
        write_data(&self.positions, output);
        println!(
            "thread {} count {} skipped {} pos/sec {:.2}",
            self.id, self.total, self.skipped, self.total as f32 / self.timer.elapsed().as_secs_f32()
        );
        self.positions.clear();
    }

    fn run_game(&mut self, position: Position, params: TunableParams, policy: &'a PolicyNetwork) {
        let mut engine = Searcher::new(position, Vec::new(), NODES_PER_MOVE, params, policy);

        // play 8 or 9 random moves
        for _ in 0..(8 + (self.rng.rand_int() % 2)) {
            let moves = engine.startpos.gen::<true>();

            if moves.is_empty() {
                return;
            }

            let mov = moves[self.rng.rand_int() as usize % moves.len()];

            engine.startstack.push(engine.startpos.hash());
            engine.startpos.make(mov, None);
        }

        if engine.startpos.gen::<true>().is_empty() {
            return;
        }

        // play out game
        loop {
            let (bm, _) = engine.search(None, 128, false, false, &mut 0, None);

            // disallow positions with >106 moves
            if engine.tree[0].moves.len() <= 106 {
                let mut training_pos = TrainingPosition::new(engine.startpos);

                for mov in engine.tree[0].moves.iter() {
                    if mov.ptr() == -1 {
                        continue;
                    }

                    let child = &engine.tree[mov.ptr() as usize];
                    let visits = child.visits();

                    training_pos.push(mov, visits);
                }

                self.positions.push(training_pos);
                self.total += 1;
            } else {
                self.skipped += 1;
            }

            engine.startstack.push(engine.startpos.hash());
            engine.startpos.make(bm, None);

            let moves = engine.startpos.gen::<true>();
            let game_state = engine.startpos.game_state(&moves, &engine.startstack);
            if game_state != GameState::Ongoing {
                break;
            }
        }
    }
}
