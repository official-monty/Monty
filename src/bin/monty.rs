use monty::{
    chess::{PolicyNetwork, Uci, ValueNetwork},
    UciLike,
};

#[repr(C)]
struct Nets(ValueNetwork, PolicyNetwork);

const NETS: Nets = unsafe { std::mem::transmute(*include_bytes!("../../resources/net.network")) };

static VALUE: ValueNetwork = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    if let Some("bench") = arg1.as_deref() {
        monty::chess::Uci::bench(4, &POLICY, &VALUE);
        return;
    }

    Uci::run(&POLICY, &VALUE);
}
