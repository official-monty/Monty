fn main() {
    let mut args = std::env::args();
    args.next();
    let buffer_size_mb = args.next().unwrap().parse().unwrap();
    let threads = args.next().unwrap().parse().unwrap();

    policy::train(
        buffer_size_mb,
        threads,
        "../binpacks/policygen6.binpack".to_string(),
        60,
        25,
    );
}
