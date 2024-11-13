fn main() {
    #[cfg(feature = "embed")]
    net::run();

    #[cfg(not(feature = "embed"))]
    nonet::run();
}

#[cfg(feature = "embed")]
mod net {
    use monty::{uci, ChessState, MctsParams, PolicyNetwork, ValueNetwork};

    static VALUE: ValueNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../value.network")) };
    static POLICY: PolicyNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../policy.network")) };

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        if let Some("bench") = arg1.as_deref() {
            uci::bench(
                ChessState::BENCH_DEPTH,
                &POLICY,
                &VALUE,
                &MctsParams::default(),
            );
            return;
        }

        uci::run(&POLICY, &VALUE);
    }
}

#[cfg(not(feature = "embed"))]
mod nonet {
    use monty::{read_into_struct_unchecked, uci, ChessState, MappedWeights, MctsParams};

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        let policy_mapped: MappedWeights<monty::PolicyNetwork> =
            unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };

        let value_mapped: MappedWeights<monty::ValueNetwork> =
            unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

        let policy = policy_mapped.data;
        let value = value_mapped.data;

        if let Some("bench") = arg1.as_deref() {
            uci::bench(
                ChessState::BENCH_DEPTH,
                policy,
                value,
                &MctsParams::default(),
            );
            return;
        }

        uci::run(policy, value);
    }
}
