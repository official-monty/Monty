fn main() {
    let mut args = std::env::args();
    args.next();
    let threads = args.next().unwrap().parse().unwrap();
    let data_path = args.next().unwrap();
    train::ataxx::train_policy(threads, &data_path);

    //train::ataxx::train_value();
}
