use crate::{
    chess::{ChessState, Move},
    mcts::{Limits, SearchHelpers, Searcher},
    MctsParams, PolicyNetwork, Tree, ValueNetwork,
};

use std::{
    io, process,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub fn run(policy: &PolicyNetwork, value: &ValueNetwork) {
    let mut prev = None;
    let mut pos = ChessState::default();
    let mut root_game_ply = 0;
    let mut params = MctsParams::default();
    let mut tree = Tree::new_mb(64, 1);
    let mut report_moves = false;
    let mut threads = 1;
    let mut move_overhead = 40;

    let mut stored_message: Option<String> = None;

    loop {
        let input = if let Some(msg) = stored_message {
            msg.clone()
        } else {
            let mut input = String::new();
            let bytes_read = io::stdin().read_line(&mut input).unwrap();

            if bytes_read == 0 {
                break;
            }

            input
        };

        stored_message = None;

        let commands = input.split_whitespace().collect::<Vec<_>>();

        let cmd = *commands.first().unwrap_or(&"oops");
        match cmd {
            "isready" => println!("readyok"),
            "setoption" => setoption(
                &commands,
                &mut params,
                &mut report_moves,
                &mut tree,
                &mut threads,
                &mut move_overhead,
            ),
            "position" => position(commands, &mut pos),
            "go" => {
                // increment game ply every time `go` is called
                root_game_ply += 2;

                go(
                    &commands,
                    &mut tree,
                    prev,
                    &pos,
                    root_game_ply,
                    &params,
                    report_moves,
                    policy,
                    value,
                    threads,
                    move_overhead,
                    &mut stored_message,
                );

                prev = Some(pos.clone());
            }
            "bench" => {
                let depth = if let Some(d) = commands.get(1) {
                    d.parse().unwrap_or(ChessState::BENCH_DEPTH)
                } else {
                    ChessState::BENCH_DEPTH
                };

                bench(depth, policy, value, &params);
            }
            "perft" => run_perft(&commands, &pos),
            "quit" => std::process::exit(0),
            "eval" => {
                println!("cp: {}", pos.get_value(value, &params));
                println!("wdl: {:.2}%", 100.0 * pos.get_value_wdl(value, &params));
            }
            "policy" => {
                let f = pos.get_policy_feats(policy);
                let mut max = f32::NEG_INFINITY;
                let mut moves = Vec::new();

                pos.map_legal_moves(|mov| {
                    let s = pos.conv_mov_to_str(mov);
                    let p = pos.get_policy(mov, &f, policy);

                    if p > max {
                        max = p;
                    }

                    moves.push((s, p));
                });

                let mut total = 0.0;

                for (_, p) in &mut moves {
                    *p = (*p - max).exp();
                    total += *p;
                }

                // Sort the moves by probability in descending order.
                moves.sort_by(|(_, p1), (_, p2)| p2.partial_cmp(p1).unwrap());

                for (s, p) in moves {
                    println!("{s} -> {:.2}%", p / total * 100.0);
                }
            }
            "d" => pos.display(policy),
            "params" => params.list_spsa(),
            "uci" => preamble(),
            "ucinewgame" => {
                prev = None;
                root_game_ply = 0;
                tree.clear(threads);
            }
            _ => {}
        }
    }
}

pub fn bench(depth: usize, policy: &PolicyNetwork, value: &ValueNetwork, params: &MctsParams) {
    let mut total_nodes = 0;
    let mut time = 0.0;

    let bench_fens = [
        "r3k2r/2pb1ppp/2pp1q2/p7/1nP1B3/1P2P3/P2N1PPP/R2QK2R w KQkq a6 0 14",
        "4rrk1/2p1b1p1/p1p3q1/4p3/2P2n1p/1P1NR2P/PB3PP1/3R1QK1 b - - 2 24",
        "r3qbrk/6p1/2b2pPp/p3pP1Q/PpPpP2P/3P1B2/2PB3K/R5R1 w - - 16 42",
        "6k1/1R3p2/6p1/2Bp3p/3P2q1/P7/1P2rQ1K/5R2 b - - 4 44",
        "8/8/1p2k1p1/3p3p/1p1P1P1P/1P2PK2/8/8 w - - 3 54",
        "7r/2p3k1/1p1p1qp1/1P1Bp3/p1P2r1P/P7/4R3/Q4RK1 w - - 0 36",
        "r1bq1rk1/pp2b1pp/n1pp1n2/3P1p2/2P1p3/2N1P2N/PP2BPPP/R1BQ1RK1 b - - 2 10",
        "3r3k/2r4p/1p1b3q/p4P2/P2Pp3/1B2P3/3BQ1RP/6K1 w - - 3 87",
        "2r4r/1p4k1/1Pnp4/3Qb1pq/8/4BpPp/5P2/2RR1BK1 w - - 0 42",
        "4q1bk/6b1/7p/p1p4p/PNPpP2P/KN4P1/3Q4/4R3 b - - 0 37",
        "2q3r1/1r2pk2/pp3pp1/2pP3p/P1Pb1BbP/1P4Q1/R3NPP1/4R1K1 w - - 2 34",
        "1r2r2k/1b4q1/pp5p/2pPp1p1/P3Pn2/1P1B1Q1P/2R3P1/4BR1K b - - 1 37",
        "r3kbbr/pp1n1p1P/3ppnp1/q5N1/1P1pP3/P1N1B3/2P1QP2/R3KB1R b KQkq b3 0 17",
        "8/6pk/2b1Rp2/3r4/1R1B2PP/P5K1/8/2r5 b - - 16 42",
        "1r4k1/4ppb1/2n1b1qp/pB4p1/1n1BP1P1/7P/2PNQPK1/3RN3 w - - 8 29",
        "8/p2B4/PkP5/4p1pK/4Pb1p/5P2/8/8 w - - 29 68",
        "3r4/ppq1ppkp/4bnp1/2pN4/2P1P3/1P4P1/PQ3PBP/R4K2 b - - 2 20",
        "5rr1/4n2k/4q2P/P1P2n2/3B1p2/4pP2/2N1P3/1RR1K2Q w - - 1 49",
        "1r5k/2pq2p1/3p3p/p1pP4/4QP2/PP1R3P/6PK/8 w - - 1 51",
        "q5k1/5ppp/1r3bn1/1B6/P1N2P2/BQ2P1P1/5K1P/8 b - - 2 34",
        "r1b2k1r/5n2/p4q2/1ppn1Pp1/3pp1p1/NP2P3/P1PPBK2/1RQN2R1 w - - 0 22",
        "r1bqk2r/pppp1ppp/5n2/4b3/4P3/P1N5/1PP2PPP/R1BQKB1R w KQkq - 0 5",
        "r1bqr1k1/pp1p1ppp/2p5/8/3N1Q2/P2BB3/1PP2PPP/R3K2n b Q - 1 12",
        "r1bq2k1/p4r1p/1pp2pp1/3p4/1P1B3Q/P2B1N2/2P3PP/4R1K1 b - - 2 19",
        "r4qk1/6r1/1p4p1/2ppBbN1/1p5Q/P7/2P3PP/5RK1 w - - 2 25",
        "r7/6k1/1p6/2pp1p2/7Q/8/p1P2K1P/8 w - - 0 32",
        "r3k2r/ppp1pp1p/2nqb1pn/3p4/4P3/2PP4/PP1NBPPP/R2QK1NR w KQkq - 1 5",
        "3r1rk1/1pp1pn1p/p1n1q1p1/3p4/Q3P3/2P5/PP1NBPPP/4RRK1 w - - 0 12",
        "5rk1/1pp1pn1p/p3Brp1/8/1n6/5N2/PP3PPP/2R2RK1 w - - 2 20",
        "8/1p2pk1p/p1p1r1p1/3n4/8/5R2/PP3PPP/4R1K1 b - - 3 27",
        "8/4pk2/1p1r2p1/p1p4p/Pn5P/3R4/1P3PP1/4RK2 w - - 1 33",
        "8/5k2/1pnrp1p1/p1p4p/P6P/4R1PK/1P3P2/4R3 b - - 1 38",
        "8/8/1p1kp1p1/p1pr1n1p/P6P/1R4P1/1P3PK1/1R6 b - - 15 45",
        "8/8/1p1k2p1/p1prp2p/P2n3P/6P1/1P1R1PK1/4R3 b - - 5 49",
        "8/8/1p4p1/p1p2k1p/P2npP1P/4K1P1/1P6/3R4 w - - 6 54",
        "8/8/1p4p1/p1p2k1p/P2n1P1P/4K1P1/1P6/6R1 b - - 6 59",
        "8/5k2/1p4p1/p1pK3p/P2n1P1P/6P1/1P6/4R3 b - - 14 63",
        "8/1R6/1p1K1kp1/p6p/P1p2P1P/6P1/1Pn5/8 w - - 0 67",
        "1rb1rn1k/p3q1bp/2p3p1/2p1p3/2P1P2N/PP1RQNP1/1B3P2/4R1K1 b - - 4 23",
        "4rrk1/pp1n1pp1/q5p1/P1pP4/2n3P1/7P/1P3PB1/R1BQ1RK1 w - - 3 22",
        "r2qr1k1/pb1nbppp/1pn1p3/2ppP3/3P4/2PB1NN1/PP3PPP/R1BQR1K1 w - - 4 12",
        "2r2k2/8/4P1R1/1p6/8/P4K1N/7b/2B5 b - - 0 55",
        "6k1/5pp1/8/2bKP2P/2P5/p4PNb/B7/8 b - - 1 44",
        "2rqr1k1/1p3p1p/p2p2p1/P1nPb3/2B1P3/5P2/1PQ2NPP/R1R4K w - - 3 25",
        "r1b2rk1/p1q1ppbp/6p1/2Q5/8/4BP2/PPP3PP/2KR1B1R b - - 2 14",
        "6r1/5k2/p1b1r2p/1pB1p1p1/1Pp3PP/2P1R1K1/2P2P2/3R4 w - - 1 36",
        "rnbqkb1r/pppppppp/5n2/8/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2",
        "2rr2k1/1p4bp/p1q1p1p1/4Pp1n/2PB4/1PN3P1/P3Q2P/2RR2K1 w - f6 0 20",
        "3br1k1/p1pn3p/1p3n2/5pNq/2P1p3/1PN3PP/P2Q1PB1/4R1K1 w - - 0 23",
        "2r2b2/5p2/5k2/p1r1pP2/P2pB3/1P3P2/K1P3R1/7R w - - 23 93",
        "5k2/4q1p1/3P1pQb/1p1B4/pP5p/P1PR4/5PP1/1K6 b - - 0 38",
        "5rk1/1rP3pp/p4n2/3Pp3/1P2Pq2/2Q4P/P5P1/R3R1K1 b - - 0 32",
        "4r1k1/4r1p1/8/p2R1P1K/5P1P/1QP3q1/1P6/3R4 b - - 0 1",
        "3qk1b1/1p4r1/1n4r1/2P1b2B/p3N2p/P2Q3P/8/1R3R1K w - - 2 39",
    ];

    let limits = Limits {
        max_time: None,
        opt_time: None,
        max_depth: depth,
        max_nodes: 1_000_000,
    };

    let mut tree = Tree::new_mb(32, 1);

    for fen in bench_fens {
        let abort = AtomicBool::new(false);
        let pos = ChessState::from_fen(fen);
        tree.try_use_subtree(&pos, &None);
        let searcher = Searcher::new(pos, &tree, params, policy, value, &abort);
        let timer = Instant::now();
        searcher.search(1, limits, false, &mut total_nodes);
        time += timer.elapsed().as_secs_f32();
        tree.clear(1);
    }

    println!(
        "Bench: {total_nodes} nodes {:.0} nps",
        total_nodes as f32 / time
    );
}

fn preamble() {
    println!("id name monty {}", env!("CARGO_PKG_VERSION"));
    println!("id author Jamie Whiting");
    println!("option name Hash type spin default 64 min 1 max 8192");
    println!("option name Threads type spin default 1 min 1 max 512");
    println!("option name UCI_Chess960 type check default false");
    println!("option name MoveOverhead type spin default 40 min 0 max 5000");
    println!("option name report_moves type button");

    #[cfg(feature = "tunable")]
    MctsParams::info(MctsParams::default());

    println!("uciok");
}

fn setoption(
    commands: &[&str],
    params: &mut MctsParams,
    report_moves: &mut bool,
    tree: &mut Tree,
    threads: &mut usize,
    move_overhead: &mut usize,
) {
    if let ["setoption", "name", "report_moves"] = commands {
        *report_moves = !*report_moves;
        return;
    }

    let (name, val) = if let ["setoption", "name", x, "value", y] = commands {
        if *x == "UCI_Chess960" {
            return;
        }

        if *x == "Threads" {
            *threads = y.parse().unwrap();
            return;
        }

        if *x == "MoveOverhead" {
            *move_overhead = y.parse().unwrap();
            return;
        }

        (*x, y.parse::<i32>().unwrap_or(0))
    } else {
        return;
    };

    if name == "Hash" {
        *tree = Tree::new_mb(val as usize, *threads);
    } else {
        params.set(name, val);
    }
}

fn position(commands: Vec<&str>, pos: &mut ChessState) {
    let mut fen = String::new();
    let mut move_list = Vec::new();
    let mut moves = false;

    for cmd in commands {
        match cmd {
            "position" | "fen" => {}
            "startpos" => fen = ChessState::STARTPOS.to_string(),
            "moves" => moves = true,
            _ => {
                if moves {
                    move_list.push(cmd);
                } else {
                    fen.push_str(&format!("{cmd} "));
                }
            }
        }
    }

    *pos = ChessState::from_fen(&fen);

    for &m in move_list.iter() {
        let mut this_mov = Move::default();

        pos.map_legal_moves(|mov| {
            if m == pos.conv_mov_to_str(mov) {
                this_mov = mov;
            }
        });

        pos.make_move(this_mov);
    }
}

#[allow(clippy::too_many_arguments)]
fn go(
    commands: &[&str],
    tree: &mut Tree,
    prev: Option<ChessState>,
    pos: &ChessState,
    root_game_ply: u32,
    params: &MctsParams,
    report_moves: bool,
    policy: &PolicyNetwork,
    value: &ValueNetwork,
    threads: usize,
    move_overhead: usize,
    stored_message: &mut Option<String>,
) {
    let mut max_nodes = i32::MAX as usize;
    let mut max_time = None;
    let mut max_depth = 256;

    let mut times = [None; 2];
    let mut incs = [None; 2];
    let mut movestogo = None;
    let mut opt_time = None;

    let mut mode = "";

    let saturating_parse = |s: &str| s.parse::<i64>().ok().map(|val| val.max(0) as u64);

    for cmd in commands {
        match *cmd {
            "nodes" => mode = "nodes",
            "movetime" => mode = "movetime",
            "depth" => mode = "depth",
            "wtime" => mode = "wtime",
            "btime" => mode = "btime",
            "winc" => mode = "winc",
            "binc" => mode = "binc",
            "movestogo" => mode = "movestogo",
            _ => match mode {
                "nodes" => max_nodes = cmd.parse().unwrap_or(max_nodes),
                "movetime" => max_time = cmd.parse().ok(),
                "depth" => max_depth = cmd.parse().unwrap_or(max_depth),
                "wtime" => times[0] = saturating_parse(cmd),
                "btime" => times[1] = saturating_parse(cmd),
                "winc" => incs[0] = saturating_parse(cmd),
                "binc" => incs[1] = saturating_parse(cmd),
                "movestogo" => movestogo = saturating_parse(cmd),
                _ => mode = "none",
            },
        }
    }

    // `go wtime <wtime> btime <btime> winc <winc> binc <binc>``
    if let Some(remaining) = times[pos.stm()] {
        let timeman =
            SearchHelpers::get_time(remaining, incs[pos.stm()], root_game_ply, movestogo, params);

        opt_time = Some(timeman.0);
        max_time = Some(timeman.1);
    }

    // `go movetime <time>`
    if let Some(max) = max_time {
        // if both movetime and increment time controls given, use
        max_time = Some(max_time.unwrap_or(u128::MAX).min(max));
    }

    // apply move overhead
    if let Some(t) = opt_time.as_mut() {
        *t = t.saturating_sub(move_overhead as u128);
    }
    if let Some(t) = max_time.as_mut() {
        *t = t.saturating_sub(move_overhead as u128);
    }

    let abort = AtomicBool::new(false);

    tree.try_use_subtree(pos, &prev);

    let limits = Limits {
        max_time,
        opt_time,
        max_depth,
        max_nodes,
    };

    std::thread::scope(|s| {
        s.spawn(|| {
            let searcher = Searcher::new(pos.clone(), tree, params, policy, value, &abort);
            let (mov, _) = searcher.search(threads, limits, true, &mut 0);
            println!("bestmove {}", pos.conv_mov_to_str(mov));

            if report_moves {
                searcher.display_moves();
            }
        });

        *stored_message = handle_search_input(&abort);
    });
}

fn run_perft(commands: &[&str], pos: &ChessState) {
    let depth = commands[1].parse().unwrap();
    let root_pos = pos.clone();
    let now = Instant::now();
    let count = root_pos.perft(depth);
    let time = now.elapsed().as_micros();
    println!(
        "perft {depth} time {} nodes {count} ({:.2} Mnps)",
        time / 1000,
        count as f32 / time as f32
    );
}

fn handle_search_input(abort: &AtomicBool) -> Option<String> {
    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();

        if bytes_read == 0 {
            process::exit(0);
        }

        match input.as_str().trim() {
            "isready" => println!("readyok"),
            "quit" => std::process::exit(0),
            "stop" => {
                abort.store(true, Ordering::Relaxed);
                return None;
            }
            _ => return Some(input),
        };
    }
}
