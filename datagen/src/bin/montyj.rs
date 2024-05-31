use datagen::{parse_args, run_datagen};
use monty::{
    shatranj::{PolicyNetwork, Shatranj}, GameRep, ValueNetwork
};

#[repr(C)]
struct Nets(ValueNetwork<768, 8>, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!("../../../resources/net.network")) };

static VALUE: ValueNetwork<768, 8> = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let args = std::env::args();
    let (threads, book, policy) = parse_args(args);

    run_datagen::<Shatranj, 112>(
        Shatranj::default_mcts_params(),
        1_000,
        threads,
        policy,
        "Shatranj",
        &POLICY,
        &VALUE,
        book,
    );
}
