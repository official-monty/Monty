use std::time::Duration;

use monty_core::POLICY_NETWORK;
use monty_engine::TunableParams;
use monty_train::{set_stop, DatagenThread};

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();

    let params = TunableParams::default();
    let policy = Box::new(POLICY_NETWORK);

    std::thread::scope(|s| {
        for i in 0..threads {
            let params = params.clone();
            let policy = &policy;
            std::thread::sleep(Duration::from_millis(10));
            s.spawn(move || {
                let mut thread = DatagenThread::new(i, params.clone(), policy);
                thread.run();
            });
        }

        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let commands = input.split_whitespace().collect::<Vec<_>>();
            if let Some(&"stop") = commands.first() {
                set_stop();
                break;
            }
        }
    });
}
