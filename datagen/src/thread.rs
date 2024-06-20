use crate::{to_slice_with_lifetime, Binpack, PolicyData, Rand};

use monty::{
    ChessState, GameState, Limits, MctsParams, PolicyNetwork, Searcher, Tree, ValueNetwork,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

pub struct DatagenThread<'a> {
    id: u32,
    rng: Rand,
    params: MctsParams,
    skipped: usize,
    total: usize,
    timer: Instant,
    vout: Arc<Mutex<BufWriter<File>>>,
    stop: &'a AtomicBool,
    book: Option<Vec<&'a str>>,
}

impl<'a> DatagenThread<'a> {
    pub fn new(
        id: u32,
        params: MctsParams,
        stop: &'a AtomicBool,
        book: Option<Vec<&'a str>>,
        vout: Arc<Mutex<BufWriter<File>>>,
    ) -> Self {
        Self {
            id,
            rng: Rand::with_seed(),
            params,
            skipped: 0,
            total: 0,
            timer: Instant::now(),
            vout,
            stop,
            book,
        }
    }

    pub fn run(
        &mut self,
        node_limit: usize,
        output_policy: bool,
        policy: &PolicyNetwork,
        value: &ValueNetwork,
    ) {
        let pout_path = format!("monty-policy-{}.data", self.rng.rand_int());
        let mut pout = if output_policy {
            Some(BufWriter::new(
                File::create(pout_path.as_str()).expect("Provide a correct path!"),
            ))
        } else {
            None
        };

        let mut prev = 0;

        loop {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            self.run_game(node_limit, &mut pout, policy, value);

            if self.total > prev + 1024 {
                prev = self.total;
                println!(
                    "thread {} count {} skipped {} pos/sec {:.2}",
                    self.id,
                    self.total,
                    self.skipped,
                    self.total as f32 / self.timer.elapsed().as_secs_f32()
                );
            }
        }
    }

    fn run_game(
        &mut self,
        node_limit: usize,
        pout: &mut Option<BufWriter<File>>,
        policy: &PolicyNetwork,
        value: &ValueNetwork,
    ) {
        let mut position = if let Some(book) = &self.book {
            let idx = self.rng.rand_int() as usize % book.len();
            ChessState::from_fen(book[idx])
        } else {
            ChessState::from_fen(ChessState::STARTPOS)
        };

        // play 8 or 9 random moves
        for _ in 0..(8 + (self.rng.rand_int() % 2)) {
            let mut moves = Vec::new();
            position.map_legal_moves(|mov| moves.push(mov));

            if moves.is_empty() {
                return;
            }

            let mov = moves[self.rng.rand_int() as usize % moves.len()];

            position.make_move(mov);
        }

        let mut moves = Vec::new();
        position.map_legal_moves(|mov| moves.push(mov));

        if moves.is_empty() {
            return;
        }

        let limits = Limits {
            max_depth: 12,
            max_nodes: node_limit,
            max_time: None,
        };

        let mut records = Vec::new();
        let mut result = 0.5;

        let mut tree = Tree::new_mb(8);

        let mut game = Binpack::new(position.clone());

        // play out game
        loop {
            let abort = AtomicBool::new(false);
            let mut searcher = Searcher::new(
                position.clone(),
                tree,
                self.params.clone(),
                policy,
                value,
                &abort,
            );

            let (bm, score) = searcher.search(limits, false, &mut 0, &None);

            game.push(position.stm(), bm, score);

            tree = searcher.tree_and_board().0;

            let mut root_count = 0;
            position.map_legal_moves(|_| root_count += 1);

            // disallow positions with >106 moves and moves when in check
            if root_count <= 112 {
                let mut policy_pos = PolicyData::new(position.clone(), bm, score);

                for action in tree[tree.root_node()].actions() {
                    policy_pos.push(action.mov().into(), action.visits());
                }

                records.push(policy_pos);
                self.total += 1;
            } else {
                self.skipped += 1;
            }

            position.make_move(bm);

            let game_state = position.game_state();
            match game_state {
                GameState::Ongoing => {}
                GameState::Draw => break,
                GameState::Lost(_) => {
                    if position.stm() == 1 {
                        result = 1.0;
                    } else {
                        result = 0.0;
                    }
                    break;
                }
                GameState::Won(_) => {
                    if position.stm() == 1 {
                        result = 0.0;
                    } else {
                        result = 1.0;
                    }
                    break;
                }
            }

            tree.clear();
        }

        if let Some(out) = pout {
            for policy in &mut records {
                policy.set_result(result);
            }

            write(&records, out);
        }

        game.set_result(result);

        let mut v = self.vout.lock().unwrap();
        let vout = v.by_ref();
        game.serialise_into(vout).unwrap();
    }
}

pub fn write<T>(input: &[T], output: &mut BufWriter<File>) {
    if input.is_empty() {
        return;
    }

    let data_slice = to_slice_with_lifetime(input);

    output
        .write_all(data_slice)
        .expect("Nothing can go wrong in unsafe code!");
}
