use crate::{Destination, Rand};

use monty::{
    chess::{ChessState, GameState},
    mcts::{Limits, MctsParams, Searcher},
    networks::{PolicyNetwork, ValueNetwork},
    tree::Tree,
};
use montyformat::{MontyFormat, MontyValueFormat, SearchData};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
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

    pub fn run(&mut self, output_policy: bool, policy: &PolicyNetwork, value: &ValueNetwork) {
        loop {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            self.run_game(policy, value, output_policy);
        }
    }

    fn run_game(&mut self, policy: &PolicyNetwork, value: &ValueNetwork, output_policy: bool) {
        let mut position = if let Some(book) = &self.book {
            let idx = self.rng.rand_int() as usize % book.len();
            ChessState::from_fen(book[idx])
        } else {
            ChessState::from_fen(ChessState::STARTPOS)
        };

        let mut moves = Vec::new();
        position.map_legal_moves(|mov| moves.push(mov));

        if moves.is_empty() {
            return;
        }

        let limits = Limits {
            max_depth: 64,
            max_nodes: 100000,
            max_time: None,
            opt_time: None,
            kld_min_gain: Some(0.000005),
        };

        let mut result = 0.5;

        let mut tree = Tree::new_mb(8, 1);
        let mut temp = 0.8;

        let pos = position.board();

        let montyformat_position = montyformat::chess::Position::from_raw(
            pos.bbs(),
            pos.stm() > 0,
            pos.enp_sq(),
            pos.rights(),
            pos.halfm(),
            pos.fullm(),
        );

        let montyformat_castling = montyformat::chess::Castling::from_raw(
            &montyformat_position,
            position.castling().rook_files(),
        );

        let mut value_game = MontyValueFormat {
            startpos: montyformat_position,
            castling: montyformat_castling,
            result: 0.5,
            moves: Vec::new(),
        };

        let mut policy_game = MontyFormat::new(montyformat_position, montyformat_castling);

        let mut total_iters = 0usize;
        let mut searches = 0;

        // play out game
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return;
            }

            let abort = AtomicBool::new(false);
            tree.set_root_position(&position);
            let searcher = Searcher::new(&tree, &self.params, policy, value, &abort);

            let (bm, score, iters) = searcher.search(1, limits, false, &mut 0, true, temp);

            searches += 1;
            total_iters += iters;

            temp *= 0.9;
            if temp <= 0.2 {
                temp = 0.0;
            }

            let best_move = montyformat::chess::Move::from(u16::from(bm));

            value_game.push(position.stm(), best_move, score);

            let mut root_count = 0;
            position.map_legal_moves(|_| root_count += 1);

            let dist = if root_count == 0 {
                None
            } else {
                let mut dist = Vec::new();

                let actions = tree[tree.root_node()].actions();

                for action in 0..tree[tree.root_node()].num_actions() {
                    let node = &tree[actions + action];
                    let mov = montyformat::chess::Move::from(u16::from(node.parent_move()));
                    dist.push((mov, node.visits()));
                }

                assert_eq!(root_count, dist.len());

                Some(dist)
            };

            let search_data = SearchData::new(best_move, score, dist);

            policy_game.push(search_data);

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

        value_game.result = result;
        policy_game.result = result;

        if self.stop.load(Ordering::Relaxed) {
            return;
        }

        let mut dest = self.dest.lock().unwrap();

        if output_policy {
            dest.push_policy(&policy_game, self.stop, searches, total_iters);
        } else {
            dest.push(&value_game, self.stop);
        }
    }
}
