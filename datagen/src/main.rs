use datagen::{parse_args, run_datagen};
use monty::{ChessState, MctsParams, Uci, read_into_struct_unchecked};

fn main() {
    let args = std::env::args();
    let opts = parse_args(args);

    let policy = unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };
    let value = unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

    Uci::bench(ChessState::BENCH_DEPTH, &policy, &value);

    let mut params = MctsParams::default();

    // value data params
    params.set("root_pst", 262);
    params.set("root_cpuct", 108);
    params.set("cpuct", 108);

    run_datagen(params, opts, &policy, &value);
}
