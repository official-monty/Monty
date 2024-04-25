use datagen::run_datagen;
use monty::{shatranj::{PolicyNetwork, Shatranj}, ValueNetwork};

#[repr(C)]
struct Nets(ValueNetwork<768, 8>, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!(concat!("../../../", env!("EVALFILE")))) };

static VALUE: ValueNetwork<768, 8> = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    args.next();

    let threads = args.next().unwrap().parse().unwrap();
    let policy = args.next() != Some("--no-policy".to_string());

    run_datagen::<Shatranj, 112>(1_000, threads, policy, "Shatranj", &POLICY, &VALUE);
}