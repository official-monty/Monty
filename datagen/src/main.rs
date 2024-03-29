fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();

    datagen::run_datagen::<monty::chess::Chess>(
        1_000,
        threads,
        &monty::chess::POLICY_NETWORK,
        &monty::chess::NNUE,
    );
}
