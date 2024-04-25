use monty::shatranj::PolicyNetwork;

fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();

    policy::train::<PolicyNetwork>(threads, "data/shatranj/whatever.data".to_string(), 10, 7);
}
