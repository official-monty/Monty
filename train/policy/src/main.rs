use policy::chess::PolicyNetwork;

fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();

    policy::train::<PolicyNetwork>(threads, "data/chess/policy5k.data".to_string(), 60, 25);
}
