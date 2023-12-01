use monty_engine::{TunableParams, POLICY_NETWORK};
use monty_train::{DatagenThread, set_stop};

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let target_positions = args.next().unwrap().parse().unwrap();

    let params = TunableParams::default();
    let policy = Box::new(POLICY_NETWORK);

    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut thread = DatagenThread::new(params.clone(), &policy);
                thread.run(target_positions);
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
