use crate::{to_slice_with_lifetime, Binpack, Destination, PolicyData, Rand};

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
};

pub struct DatagenThread<'a> {
    rng: Rand,
    params: MctsParams,
    dest: Arc<Mutex<Destination>>,
    stop: &'a AtomicBool,
    book: Option<Vec<&'a str>>,
}

impl<'a> DatagenThread<'a> {
    pub fn new(
        params: MctsParams,
        stop: &'a AtomicBool,
        book: Option<Vec<&'a str>>,
        dest: Arc<Mutex<Destination>>,
    ) -> Self {
        Self {
            rng: Rand::with_seed(),
            params,
            dest,
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

        loop {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            self.run_game(node_limit, &mut pout, policy, value);
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
            opt_time: None,
        };

        let mut records = Vec::new();
        let mut result = 0.5;

        let mut tree = Tree::new_mb(8, 1);

        let mut game = Binpack::new(position.clone());

        // play out game
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return;
            }

            let abort = AtomicBool::new(false);
            tree.try_use_subtree(&position, &None, 1);
            let searcher =
                Searcher::new(position.clone(), &tree, &self.params, policy, value, &abort);

            let (bm, score) = searcher.search(1, limits, false, &mut 0);

            game.push(position.stm(), bm, score);

            let mut root_count = 0;
            position.map_legal_moves(|_| root_count += 1);

            // disallow positions with >106 moves and moves when in check
            if root_count <= 112 {
                let mut policy_pos = PolicyData::new(position.clone(), bm, score);

                for action in tree[tree.root_node()].actions().iter() {
                    policy_pos.push(action.mov().into(), action.visits());
                }

                records.push(policy_pos);
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

            tree.clear(1);
        }

        if let Some(out) = pout {
            for policy in &mut records {
                policy.set_result(result);
            }

            write(&records, out);
        }

        game.set_result(result);

        if self.stop.load(Ordering::Relaxed) {
            return;
        }

        let mut dest = self.dest.lock().unwrap();
        dest.push(&game, self.stop);
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
