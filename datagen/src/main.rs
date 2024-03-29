fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();

    #[cfg(not(feature = "ataxx"))]
    datagen::run_datagen::<monty::chess::Chess>(
        1_000,
        threads,
    );

    #[cfg(feature = "ataxx")]
    datagen::run_datagen::<monty::ataxx::Ataxx>(
        1_000,
        threads,
    );
}
