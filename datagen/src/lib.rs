pub mod impls;
mod rng;
mod thread;

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use bulletformat::BulletFormat;
pub use rng::Rand;
pub use thread::{write, DatagenThread};

use monty::{GameRep, TunableParams};

pub trait PolicyFormat<T: GameRep> {
    const MAX_MOVES: usize;
    fn push(&mut self, mov: T::Move, visits: i16);
    fn set_result(&mut self, result: f32);
}

pub trait DatagenSupport: GameRep {
    type MoveInfo;
    type ValueData: BulletFormat;
    type PolicyData: PolicyFormat<Self>;

    fn into_policy(pos: &Self, score: f32) -> Self::PolicyData;

    fn into_value(pos: &Self, score: f32) -> Self::ValueData;
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

pub fn run_datagen<T: DatagenSupport>(
    nodes: usize,
    threads: usize,
    policy: &T::Policy,
    value: &T::Value,
) {
    let params = TunableParams::default();
    let stop_base = AtomicBool::new(false);
    let stop = &stop_base;

    std::thread::scope(|s| {
        for i in 0..threads {
            let params = params.clone();
            let policy = &policy;
            std::thread::sleep(Duration::from_millis(10));
            s.spawn(move || {
                let mut thread =
                    DatagenThread::<T>::new(i as u32, params.clone(), policy, value, stop);
                thread.run(nodes);
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
