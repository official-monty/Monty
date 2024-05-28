use monty::{
    ataxx::{PolicyNetwork, Uai},
    UciLike, ValueNetwork,
};

#[repr(C)]
struct Nets(ValueNetwork<2916, 256>, PolicyNetwork);

const NETS: Nets = unsafe { std::mem::transmute(*include_bytes!("../../resources/net.network")) };

static VALUE: ValueNetwork<2916, 256> = NETS.0;
static POLICY: PolicyNetwork = NETS.1;

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    if let Some("bench") = arg1.as_deref() {
        Uai::bench(5, &POLICY, &VALUE);
        return;
    }

    Uai::run(&POLICY, &VALUE);
}
