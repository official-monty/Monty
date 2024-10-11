fn main() {
    #[cfg(feature = "embed")]
    net::run();

    #[cfg(not(feature = "embed"))]
    nonet::run();
}

#[cfg(feature = "embed")]
mod net {
    use monty::{ChessState, MctsParams, PolicyNetwork, Uci, ValueNetwork};

    static VALUE: ValueNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../resources/value.network")) };
    static POLICY: PolicyNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../resources/policy.network")) };

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        if let Some("bench") = arg1.as_deref() {
            Uci::bench(
                ChessState::BENCH_DEPTH,
                &POLICY,
                &VALUE,
                &MctsParams::default(),
            );
            return;
        }

        Uci::run(&POLICY, &VALUE);
    }
}

#[cfg(not(feature = "embed"))]
mod nonet {
    use monty::{read_into_struct_unchecked, ChessState, MappedWeights, MctsParams, Uci};

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
            Uci::bench(
                ChessState::BENCH_DEPTH,
                policy,
                value,
                &MctsParams::default(),
            );
            return;
        }

        Uci::run(policy, value);
    }
}
