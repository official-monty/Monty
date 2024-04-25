use datagen::run_datagen;
use monty::chess::{ValueNetwork, PolicyNetwork};

#[repr(C)]
struct Nets(ValueNetwork, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../../../", env!("EVALFILE")))) };

static VALUE: ValueNetwork = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let policy = args.next() != Some("--no-policy".to_string());

    run_datagen::<monty::chess::Chess, 112>(5_000, threads, policy, "Chess", &POLICY, &VALUE);
}
