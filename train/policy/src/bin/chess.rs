use monty::chess::PolicyNetwork;

fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();

    policy::train::<PolicyNetwork>(threads, "data/chess/new-policy.data".to_string(), 30, 20);
}
