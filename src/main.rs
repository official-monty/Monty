use monty_engine::{uci, Searcher, TunableParams};

use monty_core::{Castling, PolicyNetwork, Position, POLICY_NETWORK, STARTPOS};

use std::time::Instant;

fn main() {
    // initialise engine
    let mut castling = Castling::default();
    let mut pos = Position::parse_fen(STARTPOS, &mut castling);
    let mut params = TunableParams::default();
    let mut stack = Vec::new();
    let mut tree = Vec::new();
    let mut report_moves = false;
    let policy = Box::new(POLICY_NETWORK);

    let mut prevs = None;

    let mut args = std::env::args();

    if let Some("bench") = args.nth(1).as_deref() {
        run_bench(&params, &policy);
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
            "position" => uci::position(commands, &mut pos, &mut stack, &mut prevs, &mut castling),
            "go" => tree = uci::go(
                &commands,
                tree,
                stack.clone(),
                &pos,
                &castling,
                &params,
                report_moves,
                &policy,
                &mut prevs,
            ),
            "perft" => uci::run_perft(&commands, &pos, &castling),
            "eval" => uci::eval(&pos, &policy, &castling),
            "quit" => std::process::exit(0),
            "ucinewgame" => prevs = None,
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
        let mut castling = Castling::default();
        let pos = Position::parse_fen(fen, &mut castling);
        let mut searcher = Searcher::new(castling, pos, Vec::new(), 1_000_000, params.clone(), policy);
        searcher.search(None, 5, false, false, &mut total_nodes, None);
    }

    println!(
        "Bench: {total_nodes} nodes {:.0} nps",
        total_nodes as f32 / timer.elapsed().as_secs_f32()
    );
}
