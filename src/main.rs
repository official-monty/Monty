mod attacks;
mod consts;
mod mcts;
mod moves;
mod params;
mod policy;
mod position;
mod uci;
mod value;

use mcts::Searcher;
use params::TunableParams;
use position::Position;

use std::time::Instant;

fn main() {
    // initialise engine
    let mut pos = Position::parse_fen(uci::STARTPOS);
    let mut params = TunableParams::default();
    let mut stack = Vec::new();
    let mut report_moves = false;

    // bench
    if let Some("bench") = std::env::args().nth(1).as_deref() {
        run_bench(&params);
        return;
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
            "go" => uci::go(&commands, stack.clone(), &pos, &params, report_moves),
            "perft" => uci::perft(&commands, &pos),
            "eval" => uci::eval(&pos, &params),
            "quit" => std::process::exit(0),
            _ => {}
        }
    }
}

fn run_bench(params: &TunableParams) {
    const FEN_STRING: &str = include_str!("../resources/fens.txt");

    let mut total_nodes = 0;
    let bench_fens = FEN_STRING.split('\n').collect::<Vec<&str>>();
    let timer = Instant::now();

    for fen in bench_fens {
        let pos = Position::parse_fen(fen);
        let mut searcher = Searcher::new(pos, Vec::new(), 1_000_000, params.clone());
        searcher.search(None, 5, false, false, &mut total_nodes);
    }

    println!(
        "Bench: {total_nodes} nodes {:.0} nps",
        total_nodes as f64 / timer.elapsed().as_secs_f64()
    );
}
