use monty::ataxx::PolicyNetwork;

const EPOCHS: usize = 4;
const LR_DROP: usize = 3;

fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();
    let data_path = args.next().unwrap();

    policy::train::<PolicyNetwork>(threads, data_path, EPOCHS, LR_DROP)
}
