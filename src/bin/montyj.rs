use monty::{shatranj::{Uci, PolicyNetwork}, UciLike, ValueNetwork};

#[repr(C)]
struct Nets(ValueNetwork<768, 8>, PolicyNetwork);

const NETS: Nets =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/net.network")) };

static VALUE: ValueNetwork<768, 8> = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    if let Some("bench") = arg1.as_deref() {
        Uci::bench(6, &POLICY, &VALUE);
        return;
    }

    Uci::run(&POLICY, &VALUE);
}