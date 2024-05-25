pub mod impls;
mod rng;
mod thread;

pub use rng::Rand;
pub use thread::{write, DatagenThread};

use monty::{ataxx::Ataxx, chess::Chess, shatranj::Shatranj, GameRep};

use std::{
    env::Args, fs::File, io::Read, sync::atomic::{AtomicBool, Ordering}, time::Duration
};

pub type AtaxxPolicyData = PolicyData<Ataxx, 114>;
pub type ChessPolicyData = PolicyData<Chess, 112>;
pub type ShatranjPolicyData = PolicyData<Shatranj, 112>;

#[repr(C)]
pub struct PolicyData<T: DatagenSupport, const MAX: usize> {
    pub pos: T::CompressedBoard,
    pub moves: [(u16, u16); MAX],
    pub num: usize,
    pub score: f32,
    pub result: f32,
    pub best_move: u16,
}

impl<T: DatagenSupport, const MAX: usize> PolicyData<T, MAX> {
    pub fn new(pos: T, best_move: T::Move, score: f32) -> Self {
        Self {
            pos: T::CompressedBoard::from(pos),
            moves: [(0, 0); MAX],
            num: 0,
            score,
            result: 0.0,
            best_move: best_move.into(),
        }
    }

    pub fn push(&mut self, mov: T::Move, visits: i32) {
        self.moves[self.num] = (mov.into(), visits as u16);
        self.num += 1;
    }

    pub fn set_result(&mut self, result: f32) {
        self.result = result;
    }
}

pub trait DatagenSupport: GameRep {
    type CompressedBoard: Copy + From<Self>;
    type Binpack: BinpackType<Self>;
}

pub trait BinpackType<T: GameRep>: Sized {
    fn new(pos: T) -> Self;

    fn push(&mut self, stm: usize, mov: T::Move, score: f32);

    fn set_result(&mut self, result: f32);

    fn serialise_into(&self, writer: &mut impl std::io::Write) -> std::io::Result<()>;

    fn deserialise_from(
        reader: &mut impl std::io::BufRead,
        buffer: Vec<(u16, i16)>,
    ) -> std::io::Result<Self>;
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

pub fn run_datagen<T: DatagenSupport, const MAX_MOVES: usize>(
    nodes: usize,
    threads: usize,
    use_policy: bool,
    name: &str,
    policy: &T::Policy,
    value: &T::Value,
    book: Option<String>,
) {
    println!("Generating: {name}");

    let params = T::default_mcts_params();
    params.clone().info();

    let stop_base = AtomicBool::new(false);
    let stop = &stop_base;

    let mut buf = String::new();

    let book = book.map(|path| {
        File::open(path).unwrap().read_to_string(&mut buf).unwrap();
        buf.split('\n').collect::<Vec<&str>>()
    });

    std::thread::scope(|s| {
        for i in 0..threads {
            let params = params.clone();
            std::thread::sleep(Duration::from_millis(10));
            let this_book = book.clone();
            s.spawn(move || {
                let mut thread = DatagenThread::<T>::new(i as u32, params.clone(), stop, this_book);
                thread.run::<MAX_MOVES>(nodes, use_policy, policy, value);
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

pub fn parse_args(mut args: Args) -> (usize, Option<String>, bool) {
    args.next();

    let mut threads = None;
    let mut policy = false;
    let mut book = None;

    let mut mode = 0;

    for arg in args {
        match arg.as_str() {
            "--policy" => policy = true,
            "--threads" => mode = 1,
            "--book" => mode = 2,
            _ => match mode {
                1 => {
                    threads = Some(arg.parse().expect("can't parse"));
                    mode = 0;
                },
                2 => {
                    book = Some(arg);
                    mode = 0;
                },
                _ => println!("unrecognised argument {arg}"),
            },
        }
    }

    (threads.expect("must pass thread count!"), book, policy)
}
