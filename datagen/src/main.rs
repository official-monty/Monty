mod rng;
mod thread;

use montyformat::{MontyFormat, MontyValueFormat};
use rng::Rand;
use thread::DatagenThread;

use monty::{
    chess::ChessState,
    mcts::MctsParams,
    networks::{self, PolicyNetwork, ValueNetwork},
    read_into_struct_unchecked, uci, MappedWeights,
};

use std::{
    env::Args,
    fs::File,
    io::{BufWriter, Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

fn main() {
    let mut args = std::env::args();
    args.next();

    let policy_mapped: MappedWeights<networks::PolicyNetwork> =
        unsafe { read_into_struct_unchecked(networks::PolicyFileDefaultName) };

    let value_mapped: MappedWeights<networks::ValueNetwork> =
        unsafe { read_into_struct_unchecked(networks::ValueFileDefaultName) };

    let policy = &policy_mapped.data;
    let value = &value_mapped.data;

    let params = MctsParams::default();

    if let Some(opts) = parse_args(args) {
        run_datagen(params, opts, policy, value);
    } else {
        uci::bench(ChessState::BENCH_DEPTH, policy, value, &params);
    }
}

pub fn to_slice_with_lifetime<T, U>(slice: &[T]) -> &[U] {
    let src_size = std::mem::size_of_val(slice);
    let tgt_size = std::mem::size_of::<U>();

    assert!(
        src_size % tgt_size == 0,
        "Target type size does not divide slice size!"
    );

    let len = src_size / tgt_size;
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), len) }
}

pub struct Destination {
    writer: BufWriter<File>,
    reusable_buffer: Vec<u8>,
    games: usize,
    limit: usize,
    searches: usize,
    iters: usize,
    results: [usize; 3],
}

impl Destination {
    pub fn push(&mut self, game: &MontyValueFormat, stop: &AtomicBool) {
        if stop.load(Ordering::Relaxed) {
            return;
        }

        let result = (2.0 * game.result) as usize;
        self.results[result] += 1;
        self.games += 1;
        game.serialise_into(&mut self.writer).unwrap();

        if self.games >= self.limit {
            stop.store(true, Ordering::Relaxed);
            return;
        }

        if self.games % 32 == 0 {
            self.report();
        }
    }

    pub fn push_policy(
        &mut self,
        game: &MontyFormat,
        stop: &AtomicBool,
        searches: usize,
        iters: usize,
    ) {
        if stop.load(Ordering::Relaxed) {
            return;
        }

        let result = (game.result * 2.0) as usize;
        self.results[result] += 1;
        self.games += 1;

        self.searches += searches;
        self.iters += iters;

        game.serialise_into_buffer(&mut self.reusable_buffer)
            .unwrap();
        self.writer.write_all(&self.reusable_buffer).unwrap();
        self.reusable_buffer.clear();

        if self.games >= self.limit {
            stop.store(true, Ordering::Relaxed);
            return;
        }

        if self.games % 32 == 0 {
            self.report();
        }
    }

    pub fn report(&self) {
        if self.searches != 0 {
            let average_iters = self.iters / self.searches;
            println!("average iters {average_iters}");
        }
        println!(
            "finished games {} losses {} draws {} wins {}",
            self.games, self.results[0], self.results[1], self.results[2],
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_datagen(
    params: MctsParams,
    opts: RunOptions,
    policy: &PolicyNetwork,
    value: &ValueNetwork,
) {
    println!("{opts:#?}");

    let stop_base = AtomicBool::new(false);
    let stop = &stop_base;

    let mut buf = String::new();

    let vout = File::create(opts.out_path.as_str()).unwrap();
    let vout = BufWriter::new(vout);
    let dest = Destination {
        writer: vout,
        reusable_buffer: Vec::new(),
        games: 0,
        searches: 0,
        iters: 0,
        limit: opts.games,
        results: [0; 3],
    };

    let dest_mutex = Arc::new(Mutex::new(dest));

    let book = opts.book.map(|path| {
        File::open(path).unwrap().read_to_string(&mut buf).unwrap();
        buf.trim().split('\n').collect::<Vec<&str>>()
    });

    std::thread::scope(|s| {
        for _ in 0..opts.threads {
            let params = params.clone();
            std::thread::sleep(Duration::from_millis(10));
            let this_book = book.clone();
            let this_dest = dest_mutex.clone();
            s.spawn(move || {
                let mut thread = DatagenThread::new(params.clone(), stop, this_book, this_dest);
                thread.run(opts.policy_data, policy, value);
            });
        }
    });

    let dest = dest_mutex.lock().unwrap();

    dest.report();
}

#[derive(Debug, Default)]
pub struct RunOptions {
    games: usize,
    threads: usize,
    book: Option<String>,
    policy_data: bool,
    nodes: usize,
    out_path: String,
}

pub fn parse_args(args: Args) -> Option<RunOptions> {
    let mut opts = RunOptions::default();

    let mut mode = 0;

    //opts.policy_data = true;

    for arg in args {
        match arg.as_str() {
            "bench" => return None,
            "--policy-data" => opts.policy_data = true,
            "-t" | "--threads" => mode = 1,
            "-b" | "--book" => mode = 2,
            "-n" | "--nodes" => mode = 3,
            "-o" | "--output" => mode = 4,
            "-g" | "--games" => mode = 5,
            _ => match mode {
                1 => {
                    opts.threads = arg.parse().expect("can't parse");
                    mode = 0;
                }
                2 => {
                    opts.book = Some(arg);
                    mode = 0;
                }
                3 => {
                    opts.nodes = arg.parse().expect("can't parse");
                    mode = 0;
                }
                4 => {
                    opts.out_path = arg;
                    mode = 0;
                }
                5 => {
                    opts.games = arg.parse().expect("can't parse");
                    mode = 0;
                }
                _ => println!("unrecognised argument {arg}"),
            },
        }
    }

    Some(opts)
}
