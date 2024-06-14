use monty::{ChessState, PolicyNetwork, Uci, ValueNetwork};

static VALUE: ValueNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../resources/value.network")) };
static POLICY: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../resources/policy.network")) };

fn main() {
    let mut args = std::env::args();
    let arg1 = args.nth(1);

    if let Some("bench") = arg1.as_deref() {
        Uci::bench(ChessState::BENCH_DEPTH, &POLICY, &VALUE);
        return;
    }

    Uci::run(&POLICY, &VALUE);
}
