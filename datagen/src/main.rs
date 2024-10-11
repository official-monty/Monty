use datagen::{parse_args, run_datagen};
use monty::{read_into_struct_unchecked, ChessState, MappedWeights, MctsParams, Uci};

fn main() {
    let mut args = std::env::args();
    args.next();

    let policy_mapped: MappedWeights<monty::PolicyNetwork> =
        unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };

    let value_mapped: MappedWeights<monty::ValueNetwork> =
        unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

    let policy = &policy_mapped.data;
    let value = &value_mapped.data;

    let params = MctsParams::default();

    if let Some(opts) = parse_args(args) {
        run_datagen(params, opts, policy, value);
    } else {
        Uci::bench(ChessState::BENCH_DEPTH, policy, value, &params);
    }
}
