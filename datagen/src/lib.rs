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
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
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

#[allow(clippy::too_many_arguments)]
pub fn run_datagen(
    params: MctsParams,
    opts: RunOptions,
    policy: &PolicyNetwork,
    value: &ValueNetwork,
) {
    let stop_base = AtomicBool::new(false);
    let stop = &stop_base;

    let mut buf = String::new();

    let book = opts.book.map(|path| {
        File::open(path).unwrap().read_to_string(&mut buf).unwrap();
        buf.split('\n').collect::<Vec<&str>>()
    });

    std::thread::scope(|s| {
        for i in 0..opts.threads {
            let params = params.clone();
            std::thread::sleep(Duration::from_millis(10));
            let this_book = book.clone();
            s.spawn(move || {
                let mut thread = DatagenThread::new(i as u32, params.clone(), stop, this_book);
                thread.run(opts.nodes, opts.policy_data, policy, value);
            });
        }

        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let commands = input.split_whitespace().collect::<Vec<_>>();
            if let Some(&"stop") = commands.first() {
                stop.store(true, Ordering::Relaxed);
                break;
            }
        }
    });
}

#[derive(Default)]
pub struct RunOptions {
    threads: usize,
    book: Option<String>,
    policy_data: bool,
    nodes: usize,
}

pub fn parse_args(mut args: Args) -> RunOptions {
    args.next();

    let mut opts = RunOptions::default();

    let mut mode = 0;

    for arg in args {
        match arg.as_str() {
            "--policy-data" => opts.policy_data = true,
            "--threads" => mode = 1,
            "--book" => mode = 2,
            "--nodes" => mode = 3,
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
                _ => println!("unrecognised argument {arg}"),
            },
        }
    }

    opts
}
