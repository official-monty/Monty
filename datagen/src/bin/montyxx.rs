use datagen::{parse_args, run_datagen};
use monty::{
    ataxx::{Ataxx, PolicyNetwork},
    ValueNetwork,
};

#[repr(C)]
struct Nets(ValueNetwork<2916, 256>, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!("../../../resources/net.network")) };

static VALUE: ValueNetwork<2916, 256> = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    let (threads, book, policy) = parse_args(args);

    run_datagen::<Ataxx, 114>(1_000, threads, policy, "Ataxx", &POLICY, &VALUE, book);
}
