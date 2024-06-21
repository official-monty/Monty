mod dataformat;
mod rng;
mod thread;

pub use dataformat::{Binpack, CompressedChessBoard, PolicyData};
pub use rng::Rand;
pub use thread::{write, DatagenThread};

use monty::{MctsParams, PolicyNetwork, ValueNetwork};

use std::{
    env::Args,
    fs::File,
    io::{BufWriter, Read},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

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
    games: usize,
    limit: usize,
    results: [usize; 3],
}

impl Destination {
    pub fn push(&mut self, game: &Binpack, stop: &AtomicBool) {
        if stop.load(Ordering::SeqCst) {
            return;
        }

        let result = usize::from(game.result());
        self.results[result] += 1;
        self.games += 1;
        game.serialise_into(&mut self.writer).unwrap();

        if self.games >= self.limit {
            stop.store(true, Ordering::SeqCst);
            return;
        }

        if self.games % 32 == 0 {
            self.report();
        }
    }

    pub fn report(&self) {
        println!(
            "finished games {} losses {} draws {} wins {}",
            self.games,
            self.results[0],
            self.results[1],
            self.results[2],
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
        games: 0,
        limit: opts.games,
        results: [0; 3],
    };

    let dest_mutex = Arc::new(Mutex::new(dest));

    let book = opts.book.map(|path| {
        File::open(path).unwrap().read_to_string(&mut buf).unwrap();
        buf.split('\n').collect::<Vec<&str>>()
    });

    std::thread::scope(|s| {
        for _ in 0..opts.threads {
            let params = params.clone();
            std::thread::sleep(Duration::from_millis(10));
            let this_book = book.clone();
            let this_dest = dest_mutex.clone();
            s.spawn(move || {
                let mut thread = DatagenThread::new(
                    params.clone(),
                    stop,
                    this_book,
                    this_dest,
                );
                thread.run(opts.nodes, opts.policy_data, policy, value);
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
