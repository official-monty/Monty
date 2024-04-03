use monty::chess::PolicyNetwork;

const EPOCHS: usize = 10;
const LR_DROP: usize = 7;

fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();
    let data_path = args.next().unwrap();

    policy::train::<PolicyNetwork>(threads, data_path, EPOCHS, LR_DROP);
}
