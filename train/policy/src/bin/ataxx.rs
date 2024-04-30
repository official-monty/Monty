use monty::ataxx::PolicyNetwork;

fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();

    policy::train::<PolicyNetwork>(threads, "data/ataxx/blah.data".to_string(), 30, 20);
}
