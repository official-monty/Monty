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

pub struct Uci;

impl Uci {
    const FEN_STRING: &'static str = include_str!("../resources/bench.txt");

    pub fn options() {
        println!("option name UCI_Chess960 type check default false");
    }

    pub fn run(policy: &PolicyNetwork, value: &ValueNetwork) {
        let mut prev = None;
        let mut pos = ChessState::default();
        let mut root_game_ply = 0;
        let mut params = MctsParams::default();
        let mut tree = Tree::new_mb(64);
        let mut report_moves = false;

        let mut stored_message: Option<String> = None;

        loop {
            let input = if let Some(msg) = stored_message {
                msg.clone()
            } else {
                let mut input = String::new();
                let bytes_read = io::stdin().read_line(&mut input).unwrap();

                // got EOF, exit (for OpenBench).
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
                "setoption" => setoption(&commands, &mut params, &mut report_moves, &mut tree),
                "position" => position(commands, &mut pos, &mut prev, &mut tree),
                "go" => {
                    // increment game ply every time `go` is called
                    root_game_ply += 2;

                    let res = go(
                        &commands,
                        tree,
                        prev,
                        &pos,
                        root_game_ply,
                        &params,
                        report_moves,
                        policy,
                        value,
                        &mut stored_message,
                    );

                    tree = res.0;
                    prev = Some(res.1);
                }
                "perft" => run_perft(&commands, &pos),
                "quit" => std::process::exit(0),
                "eval" => {
                    println!("cp: {}", pos.get_value(value, &params));
                    println!("wdl: {:.2}%", 100.0 * pos.get_value_wdl(value, &params));
                }
                "policy" => {
                    let f = pos.get_policy_feats();
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

                    moves.sort_by_key(|(_, p)| (p * 1000.0) as u32);

                    for (s, p) in moves {
                        println!("{s} -> {:.2}%", p / total * 100.0);
                    }
                }
                "tree" => {
                    let u = tree.len();
                    let c = tree.cap();
                    let pct = u as f32 * 100.0 / c as f32;
                    println!("filled {u}/{c} ({pct:.2}%)");

                    let depth = commands.get(1).unwrap_or(&"5").parse().unwrap_or(5);
                    tree.display(tree.root_node(), depth);
                }
                "d" => pos.display(policy),
                "params" => params.list_spsa(),
                "uci" => preamble(),
                "ucinewgame" => {
                    prev = None;
                    root_game_ply = 0;
                    tree.clear();
                }
                _ => {}
            }
        }
    }

    pub fn bench(depth: usize, policy: &PolicyNetwork, value: &ValueNetwork) {
        let params = MctsParams::default();
        let mut total_nodes = 0;
        let bench_fens = Self::FEN_STRING.split('\n').collect::<Vec<&str>>();
        let mut time = 0.0;

        let limits = Limits {
            max_time: None,
            opt_time: None,
            max_depth: depth,
            max_nodes: 1_000_000,
        };

        let mut tree = Tree::new_mb(32);
        let abort = AtomicBool::new(false);

        for fen in bench_fens {
            let pos = ChessState::from_fen(fen);
            let mut searcher = Searcher::new(pos, tree, params.clone(), policy, value, &abort);
            let timer = Instant::now();
            searcher.search(limits, false, &mut total_nodes, &None);
            time += timer.elapsed().as_secs_f32();
            tree = searcher.tree_and_board().0;
            tree.clear();
        }

        println!(
            "Bench: {total_nodes} nodes {:.0} nps",
            total_nodes as f32 / time
        );
    }
}

fn preamble() {
    println!("id name monty {}", env!("CARGO_PKG_VERSION"));
    println!("id author Jamie Whiting");
    println!("option name Hash type spin default 64 min 1 max 8192");
    println!("option name report_moves type button");
    Uci::options();
    MctsParams::info(MctsParams::default());
    println!("uciok");
}

fn setoption(commands: &[&str], params: &mut MctsParams, report_moves: &mut bool, tree: &mut Tree) {
    if let ["setoption", "name", "report_moves"] = commands {
        *report_moves = !*report_moves;
        return;
    }

    let (name, val) = if let ["setoption", "name", x, "value", y] = commands {
        if *x == "UCI_Chess960" {
            return;
        }

        (*x, y.parse::<i32>().unwrap_or(0))
    } else {
        return;
    };

    if name == "Hash" {
        *tree = Tree::new_mb(val as usize);
    } else {
        params.set(name, val);
    }
}

fn position(
    commands: Vec<&str>,
    pos: &mut ChessState,
    prev: &mut Option<ChessState>,
    tree: &mut Tree,
) {
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

    tree.try_use_subtree(pos, prev);
    *prev = Some(pos.clone());
}

#[allow(clippy::too_many_arguments)]
fn go(
    commands: &[&str],
    tree: Tree,
    prev: Option<ChessState>,
    pos: &ChessState,
    root_game_ply: u32,
    params: &MctsParams,
    report_moves: bool,
    policy: &PolicyNetwork,
    value: &ValueNetwork,
    stored_message: &mut Option<String>,
) -> (Tree, ChessState) {
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
    if let Some(remaining) = times[pos.tm_stm()] {
        let timeman = SearchHelpers::get_time(
            remaining,
            incs[pos.stm()],
            root_game_ply,
            movestogo,
            &params,
        ); // returns a pair of (u128, u128)

        opt_time = Some(timeman.0);
        max_time = Some(timeman.1);
    }

    // `go movetime <time>`
    if let Some(max) = max_time {
        // if both movetime and increment time controls given, use
        max_time = Some(max_time.unwrap_or(u128::MAX).min(max));
    }

    // 20ms move overhead
    if let Some(t) = opt_time.as_mut() {
        *t = t.saturating_sub(20);
    }
    if let Some(t) = max_time.as_mut() {
        *t = t.saturating_sub(20);
    }

    let abort = AtomicBool::new(false);

    let mut searcher = Searcher::new(pos.clone(), tree, params.clone(), policy, value, &abort);

    let limits = Limits {
        max_time,
        opt_time,
        max_depth,
        max_nodes,
    };

    std::thread::scope(|s| {
        s.spawn(|| {
            let (mov, _) = searcher.search(limits, true, &mut 0, &prev);
            println!("bestmove {}", pos.conv_mov_to_str(mov));

            if report_moves {
                searcher.display_moves();
            }
        });

        *stored_message = handle_search_input(&abort);
    });

    searcher.tree_and_board()
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

        // got EOF, exit (for OpenBench).
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
