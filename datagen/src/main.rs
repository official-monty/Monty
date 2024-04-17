use datagen::{DatagenSupport, DatagenThread};

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let policy = args.next() != Some("--no-policy".to_string());

    #[cfg(not(feature = "ataxx"))]
    #[cfg(not(feature = "shatranj"))]
    run_datagen::<monty::chess::Chess, 112>(1_000, threads, policy, "Chess");

    #[cfg(feature = "ataxx")]
    run_datagen::<monty::ataxx::Ataxx, 114>(1_000, threads, policy, "Ataxx");

    #[cfg(feature = "shatranj")]
    run_datagen::<monty::shatranj::Shatranj>(1_000, threads, policy, "Shatranj");
}

fn run_datagen<T: DatagenSupport, const MAX_MOVES: usize>(
    nodes: usize,
    threads: usize,
    policy: bool,
    name: &str,
) {
    println!("Generating: {name}");

    let mut params = T::default_mcts_params();

    params.set("root_cpuct", 3.0);
    params.set("root_pst", 5.0);

    params.clone().info();

    let stop_base = AtomicBool::new(false);
    let stop = &stop_base;

    std::thread::scope(|s| {
        for i in 0..threads {
            let params = params.clone();
            std::thread::sleep(Duration::from_millis(10));
            s.spawn(move || {
                let mut thread = DatagenThread::<T>::new(i as u32, params.clone(), stop);
                thread.run::<MAX_MOVES>(nodes, policy);
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
