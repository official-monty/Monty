use monty::{chess::{PackedValueNetwork, PolicyNetwork, Uci}, UciLike};

#[repr(C)]
struct Nets(PackedValueNetwork, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/net.network")) };

static VALUE: PackedValueNetwork = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    let value = VALUE.unpack();

    if let Some("bench") = arg1.as_deref() {
        monty::chess::Uci::bench(4, &POLICY, &value);
        return;
    }

    Uci::run(&POLICY, &value);
}