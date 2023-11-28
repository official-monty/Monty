use crate::{
    search::{mcts::Searcher, params::TunableParams, policy::PolicyNetwork},
    state::{moves::Move, position::{GameState, Position}},
    train::rng::Rand, uci::STARTPOS,
};

#[derive(Clone)]
pub struct TrainingPosition {
    pub position: Position,
    pub moves: Vec<(Move, i32)>,
}

pub fn run_datagen(
    threads: usize,
    num_positions: usize,
    params: TunableParams,
    policy: &PolicyNetwork,
) -> Vec<TrainingPosition> {
    std::thread::scope(|s|
        (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut thread = DatagenThread::new(params.clone(), policy);
                    thread.run(num_positions);
                    thread.positions
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|p| p.join().unwrap())
            .collect::<Vec<_>>()
            .concat()
    )
}

pub struct DatagenThread<'a> {
    id: u32,
    rng: Rand,
    params: TunableParams,
    policy: &'a PolicyNetwork,
    positions: Vec<TrainingPosition>,
}

impl<'a> DatagenThread<'a> {
    fn new(params: TunableParams, policy: &'a PolicyNetwork) -> Self {
        let mut rng = Rand::with_seed();
        Self {
            id: rng.rand_int(),
            rng,
            params,
            policy,
            positions: Vec::new(),
        }
    }

    fn run(&mut self, num_positions: usize) {
        let position = Position::parse_fen(STARTPOS);

        while self.positions.len() < num_positions {
            self.run_game(position, self.params.clone(), self.policy);
        }
    }

    fn run_game(
        &mut self,
        position: Position,
        params: TunableParams,
        policy: &'a PolicyNetwork,
    ) {
        let mut engine = Searcher::new(position, Vec::new(), 5_000, params, policy);

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
            let (bm, _) = engine.search(None, 128, false, false, &mut 0);

            let mut training_pos = TrainingPosition {
                position: engine.startpos,
                moves: Vec::new(),
            };

            for mov in engine.tree[0].moves.iter() {
                if mov.ptr() == -1 {
                    continue;
                }

                let child = &engine.tree[mov.ptr() as usize];
                let visits = child.visits();

                training_pos.moves.push((*mov, visits));
            }

            self.positions.push(training_pos);
            if self.positions.len() % 1024 == 0 {
                println!("thread {} count {}", self.id, self.positions.len());
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
