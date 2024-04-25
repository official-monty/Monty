use datagen::run_datagen;
use monty::{shatranj::Shatranj, ValueNetwork};

static VALUE: ValueNetwork<768, 8> =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../../../", env!("EVALFILE")))) };

static POLICY: () = ();

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let policy = args.next() != Some("--no-policy".to_string());

    run_datagen::<Shatranj, 112>(1_000, threads, policy, "Shatranj", &POLICY, &VALUE);
}