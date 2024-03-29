use monty::{chess, TunableParams, UciLike};

fn main() {
    let mut args = std::env::args();
    let params = TunableParams::default();

    if let Some("bench") = args.nth(1).as_deref() {
        chess::Uci::bench(5, &chess::POLICY_NETWORK, &chess::NNUE, &params);
        return;
    }

    chess::Uci::run(&chess::POLICY_NETWORK, &chess::NNUE);
}
