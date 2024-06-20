use datagen::{parse_args, run_datagen};
use monty::{read_into_struct_unchecked, ChessState, MctsParams, Uci};

fn main() {
    let mut args = std::env::args();
    args.next();

    let policy = unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };
    let value = unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

    let mut params = MctsParams::default();

    // value data params
    params.set("root_pst", 262);
    params.set("root_cpuct", 108);
    params.set("cpuct", 108);

    if let Some(opts) = parse_args(args) {
        run_datagen(params, opts, &policy, &value);
    } else {
        Uci::bench(ChessState::BENCH_DEPTH, &policy, &value);
    }
}
