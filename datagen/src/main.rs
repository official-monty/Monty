fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let policy = args.next() != Some("--no-policy".to_string());

    #[cfg(not(feature = "ataxx"))]
    datagen::run_datagen::<monty::chess::Chess>(1_000, threads, policy);

    #[cfg(feature = "ataxx")]
    datagen::run_datagen::<monty::ataxx::Ataxx>(1_000, threads, policy);
}
