fn main() {
    #[cfg(not(feature = "nonet"))]
    net::run();

    #[cfg(feature = "nonet")]
    nonet::run();
}

#[cfg(not(feature = "nonet"))]
mod net {
    use monty::{ChessState, PolicyNetwork, Uci, ValueNetwork};

    static VALUE: ValueNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../resources/value.network")) };
    static POLICY: PolicyNetwork =
        unsafe { std::mem::transmute(*include_bytes!("../resources/policy.network")) };

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        if let Some("bench") = arg1.as_deref() {
            Uci::bench(ChessState::BENCH_DEPTH, &POLICY, &VALUE);
            return;
        }

        Uci::run(&POLICY, &VALUE);
    }
}

#[cfg(feature = "nonet")]
mod nonet {
    use monty::{ChessState, Uci, read_into_struct_unchecked};

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        let policy = unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };

        let value = unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

        if let Some("bench") = arg1.as_deref() {
            Uci::bench(ChessState::BENCH_DEPTH, &policy, &value);
            return;
        }

        Uci::run(&policy, &value);
    }
}
