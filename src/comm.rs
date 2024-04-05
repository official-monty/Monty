use crate::{
    game::GameRep,
    mcts::{Limits, MctsParams, Searcher, Tree},
};

use std::time::Instant;

pub trait UciLike: Sized {
    type Game: GameRep;
    const NAME: &'static str;
    const NEWGAME: &'static str;
    const OK: &'static str;
    const FEN_STRING: &'static str;

    fn options();

    fn run() {
        let mut prev = None;
        let mut pos = Self::Game::default();
        let mut params = Self::Game::default_mcts_params();
        let mut tree = Tree::new_mb(64);
        let mut report_moves = false;

        loop {
            let mut input = String::new();
            let bytes_read = std::io::stdin().read_line(&mut input).unwrap();

            if bytes_read == 0 {
                break;
            }

            let commands = input.split_whitespace().collect::<Vec<_>>();

            let cmd = *commands.first().unwrap_or(&"oops");
            match cmd {
                "isready" => println!("readyok"),
                "setoption" => setoption(&commands, &mut params, &mut report_moves, &mut tree),
                "position" => position(commands, &mut pos),
                "go" => {
                    let res = go(&commands, tree, prev, &pos, &params, report_moves);

                    tree = res.0;
                    prev = Some(res.1);
                }
                "perft" => run_perft::<Self::Game>(&commands, &pos),
                "quit" => std::process::exit(0),
                "eval" => println!("value: {}%", 100.0 * pos.get_value()),
                "policy" => {
                    let f = pos.get_policy_feats();
                    let mut max = f32::NEG_INFINITY;
                    let mut moves = Vec::new();

                    pos.map_legal_moves(|mov| {
                        let s = pos.conv_mov_to_str(mov);
                        let p = pos.get_policy(mov, &f);

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
                    println!("info string filled {u}/{c} ({pct:.2}%)");

                    let depth = commands.get(1).unwrap_or(&"5").parse().unwrap_or(5);
                    tree.display::<Self::Game>(tree.root_node(), depth);
                }
                "d" => println!("{pos}"),
                _ => {
                    if cmd == Self::NAME {
                        preamble::<Self>();
                    } else if cmd == Self::NEWGAME {
                        prev = None;
                        tree.clear();
                    }
                }
            }
        }
    }

    fn bench(depth: usize) {
        let params = Self::Game::default_mcts_params();
        let mut total_nodes = 0;
        let bench_fens = Self::FEN_STRING.split('\n').collect::<Vec<&str>>();
        let timer = Instant::now();

        let limits = Limits {
            max_time: None,
            max_depth: depth,
            max_nodes: 1_000_000,
        };

        let mut tree = Tree::new_mb(4);

        for fen in bench_fens {
            let pos = Self::Game::from_fen(fen);
            let mut searcher = Searcher::new(pos, tree, params.clone());
            searcher.search(limits, false, &mut total_nodes, &None);
            tree = searcher.tree_and_board().0;
            tree.clear();
        }

        println!(
            "Bench: {total_nodes} nodes {:.0} nps",
            total_nodes as f32 / timer.elapsed().as_secs_f32()
        );
    }
}

fn preamble<T: UciLike>() {
    println!("id name monty {}", env!("CARGO_PKG_VERSION"));
    println!("id author Jamie Whiting");
    println!("option name Hash type spin default 64 min 1 max 8192");
    println!("option name report_moves type button");
    T::options();
    MctsParams::info(T::Game::default_mcts_params());
    println!("{}", T::OK);
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
        params.set(name, val as f32 / 100.0);
    }
}

fn position<T: GameRep>(commands: Vec<&str>, pos: &mut T) {
    let mut fen = String::new();
    let mut move_list = Vec::new();
    let mut moves = false;

    for cmd in commands {
        match cmd {
            "position" | "fen" => {}
            "startpos" => fen = T::STARTPOS.to_string(),
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

    *pos = T::from_fen(&fen);

    for &m in move_list.iter() {
        let mut this_mov = T::Move::default();

        pos.map_legal_moves(|mov| {
            if m == pos.conv_mov_to_str(mov) {
                this_mov = mov;
            }
        });

        pos.make_move(this_mov);
    }
}

#[allow(clippy::too_many_arguments)]
fn go<T: GameRep>(
    commands: &[&str],
    tree: Tree,
    prev: Option<T>,
    pos: &T,
    params: &MctsParams,
    report_moves: bool,
) -> (Tree, T) {
    let mut max_nodes = 10_000_000;
    let mut max_time = None;
    let mut max_depth = 256;

    let mut times = [None; 2];
    let mut incs = [None; 2];
    let mut movestogo = 30;

    let mut mode = "";

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
                "wtime" => times[0] = Some(cmd.parse().unwrap_or(0)),
                "btime" => times[1] = Some(cmd.parse().unwrap_or(0)),
                "winc" => incs[0] = Some(cmd.parse().unwrap_or(0)),
                "binc" => incs[1] = Some(cmd.parse().unwrap_or(0)),
                "movestogo" => movestogo = cmd.parse().unwrap_or(30),
                _ => mode = "none",
            },
        }
    }

    let mut time = None;

    // `go wtime <wtime> btime <btime> winc <winc> binc <binc>``
    if let Some(t) = times[pos.tm_stm()] {
        let mut base = t / movestogo.max(1);

        if let Some(i) = incs[pos.tm_stm()] {
            base += i * 3 / 4;
        }

        time = Some(base);
    }

    // `go movetime <time>`
    if let Some(max) = max_time {
        // if both movetime and increment time controls given, use
        time = Some(time.unwrap_or(u128::MAX).min(max));
    }

    // 5ms move overhead
    if let Some(t) = time.as_mut() {
        *t = t.saturating_sub(5);
    }

    let mut searcher = Searcher::new(pos.clone(), tree, params.clone());

    let limits = Limits {
        max_time: time,
        max_depth,
        max_nodes,
    };

    let (mov, _) = searcher.search(limits, true, &mut 0, &prev);

    println!("bestmove {}", pos.conv_mov_to_str(mov));

    if report_moves {
        searcher.display_moves();
    }

    searcher.tree_and_board()
}

fn run_perft<T: GameRep>(commands: &[&str], pos: &T) {
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
