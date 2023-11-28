mod search;
mod state;
mod train;
mod uci;

use search::{
    mcts::Searcher,
    params::TunableParams,
    policy::{PolicyNetwork, POLICY_NETWORK},
};
use state::position::Position;
use train::run_training;

use std::time::Instant;

fn main() {
    // initialise engine
    let mut pos = Position::parse_fen(uci::STARTPOS);
    let mut params = TunableParams::default();
    let mut stack = Vec::new();
    let mut report_moves = false;
    let mut policy = Box::new(POLICY_NETWORK);

    let mut args = std::env::args();

    match args.nth(1).as_deref() {
        Some("bench") => {
            run_bench(&params, &policy);
            return;
        }
        Some("train") => {
            let arg = args.next().unwrap();
            run_training(arg.parse().unwrap(), params, &mut policy);
            return;
        }
        _ => {}
    }

    // main uci loop
    loop {
        let mut input = String::new();
        let bytes_read = std::io::stdin().read_line(&mut input).unwrap();

        // got EOF, exit.
        if bytes_read == 0 {
            break;
        }

        let commands = input.split_whitespace().collect::<Vec<_>>();

        match *commands.first().unwrap_or(&"oops") {
            "uci" => uci::preamble(),
            "isready" => uci::isready(),
            "setoption" => uci::setoption(&commands, &mut params, &mut report_moves),
            "position" => uci::position(commands, &mut pos, &mut stack),
            "go" => uci::go(
                &commands,
                stack.clone(),
                &pos,
                &params,
                report_moves,
                &policy,
            ),
            "perft" => uci::perft(&commands, &pos),
            "eval" => uci::eval(&pos, &params, &policy),
            "quit" => std::process::exit(0),
            _ => {}
        }
    }
}

fn run_bench(params: &TunableParams, policy: &PolicyNetwork) {
    const FEN_STRING: &str = include_str!("../resources/fens.txt");

    let mut total_nodes = 0;
    let bench_fens = FEN_STRING.split('\n').collect::<Vec<&str>>();
    let timer = Instant::now();

    for fen in bench_fens {
        let pos = Position::parse_fen(fen);
        let mut searcher = Searcher::new(pos, Vec::new(), 1_000_000, params.clone(), policy);
        searcher.search(None, 5, false, false, &mut total_nodes);
    }

    println!(
        "Bench: {total_nodes} nodes {:.0} nps",
        total_nodes as f32 / timer.elapsed().as_secs_f32()
    );
}
