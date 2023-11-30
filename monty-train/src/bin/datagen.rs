use monty_engine::{TunableParams, POLICY_NETWORK};
use monty_train::DatagenThread;

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

    });
}
