use datagen::{parse_args, run_datagen};
use monty::{
    chess::{PolicyNetwork, ValueNetwork},
    UciLike,
};

#[repr(C)]
struct Nets(ValueNetwork, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!("../../../resources/net.network")) };

static VALUE: ValueNetwork = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let args = std::env::args();
    let (threads, book, policy) = parse_args(args);

    monty::chess::Uci::bench(4, &POLICY, &VALUE);

    if let Some(path) = &book {
        println!("Using book: {path}")
    } else {
        println!("Not using a book.")
    }

    run_datagen::<monty::chess::Chess, 112>(5_000, threads, policy, "Chess", &POLICY, &VALUE, book);
}
