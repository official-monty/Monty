use crate::{to_slice_with_lifetime, DatagenSupport, PolicyFormat, Rand};

use bulletformat::BulletFormat;
use monty::{GameState, Limits, MctsParams, Searcher, Tree};

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub struct DatagenThread<'a, T: DatagenSupport> {
    id: u32,
    rng: Rand,
    params: MctsParams,
    skipped: usize,
    total: usize,
    timer: Instant,
    stop: &'a AtomicBool,
    marker: std::marker::PhantomData<T>,
}

impl<'a, T: DatagenSupport> DatagenThread<'a, T> {
    pub fn new(id: u32, params: MctsParams, stop: &'a AtomicBool) -> Self {
        Self {
            id,
            rng: Rand::with_seed(),
            params,
            skipped: 0,
            total: 0,
            timer: Instant::now(),
            stop,
            marker: std::marker::PhantomData,
        }
    }

    pub fn run(&mut self, node_limit: usize, policy: bool) {
        let pout_path = format!("monty-policy-{}.data", self.rng.rand_int());
        let vout_path = format!("monty-value-{}.data", self.rng.rand_int());
        let mut vout =
            BufWriter::new(File::create(vout_path.as_str()).expect("Provide a correct path!"));
        let mut pout = if policy {
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

            self.run_game(node_limit, &mut pout, &mut vout);

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
        vout: &mut BufWriter<File>,
    ) {
        let mut position = T::from_fen(T::STARTPOS);

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

        // play out game
        loop {
            let mut searcher = Searcher::new(position.clone(), tree, self.params.clone());

            let (bm, score) = searcher.search(limits, false, &mut 0, &None);

            tree = searcher.tree_and_board().0;

            let mut root_count = 0;
            position.map_legal_moves(|_| root_count += 1);

            // disallow positions with >106 moves and moves when in check
            if root_count <= T::PolicyData::MAX_MOVES {
                let mut policy_pos = T::into_policy(&position, score);
                let value_pos = T::into_value(&position, score);

                tree.map_children(tree.root_node(), |_, child| {
                    policy_pos.push(child.mov().into(), child.visits() as i16);
                });

                records.push((policy_pos, value_pos, position.stm()));
                self.total += 1;
            } else {
                self.skipped += 1;
            }

            position.make_move(bm);

            let game_state = position.game_state();
            match game_state {
                GameState::Ongoing => {}
                GameState::Draw => break,
                GameState::Lost => {
                    if position.stm() == 1 {
                        result = 1.0;
                    } else {
                        result = 0.0;
                    }
                    break;
                }
                GameState::Won => {
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

        let mut policies = Vec::new();
        let mut values = Vec::new();

        for (mut policy, mut value, stm) in records {
            let this_result = if result == 0.5 {
                0.5
            } else if stm as f64 == result {
                0.0
            } else {
                1.0
            };

            policy.set_result(this_result);
            value.set_result(this_result);

            policies.push(policy);
            values.push(value);
        }

        if let Some(out) = pout {
            write(&policies, out);
        }

        write(&values, vout);
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
