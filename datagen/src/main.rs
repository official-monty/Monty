use datagen::{parse_args, run_datagen};
use monty::{ChessState, MctsParams, PolicyNetwork, Uci, ValueNetwork};

static VALUE: ValueNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/value.network")) };
static POLICY: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/policy.network")) };

fn main() {
    let args = std::env::args();
    let (threads, book, policy) = parse_args(args);

    Uci::bench(ChessState::BENCH_DEPTH, &POLICY, &VALUE);

    if let Some(path) = &book {
        println!("Using book: {path}")
    } else {
        println!("Not using a book.")
    }

    let mut params = MctsParams::default();

    // value data params
    params.set("root_pst", 262);
    params.set("root_cpuct", 108);
    params.set("cpuct", 108);

    run_datagen(
        params, 5_000, threads, policy, "Chess", &POLICY, &VALUE, book,
    );
}
