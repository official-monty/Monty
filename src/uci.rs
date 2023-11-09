use crate::Position;

use std::time::Instant;

pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn preamble() {
    println!("id name monty {}", env!("CARGO_PKG_VERSION"));
    println!("id author Jamie Whiting");
    println!("uciok");
}

pub fn set_position(commands: Vec<&str>, pos: &mut Position, stack: &mut Vec<u64>) {
    let mut fen = String::new();
    let mut move_list = Vec::new();
    let mut moves = false;

    for cmd in commands {
        match cmd {
            "position" | "startpos" | "fen" => {}
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

    *pos = Position::parse_fen(if fen.is_empty() { STARTPOS } else { &fen });
    stack.clear();

    for m in move_list {
        //stack.push(pos.hash());
        let possible_moves = pos.gen();

        for mov in possible_moves.iter() {
            if m == mov.to_uci() {
                pos.make(*mov);
            }
        }
    }
}

pub fn run_perft(commands: &[&str], pos: &Position) {
    let depth = commands[1].parse().unwrap();
    let now = Instant::now();
    let count = perft::<true, true>(pos, depth);
    let time = now.elapsed().as_micros();
    println!(
        "perft {depth} time {} nodes {count} ({:.2} Mnps)",
        time / 1000,
        count as f64 / time as f64
    );
}

#[must_use]
fn perft<const ROOT: bool, const BULK: bool>(pos: &Position, depth: u8) -> u64 {
    let moves = pos.gen();

    if BULK && !ROOT && depth == 1 {
        return moves.len as u64;
    }

    let mut positions = 0;
    let leaf = depth == 1;

    for m_idx in 0..moves.len {
        let mut tmp = *pos;
        tmp.make(moves.list[m_idx]);

        let num = if !BULK && leaf {
            1
        } else {
            perft::<false, BULK>(&tmp, depth - 1)
        };
        positions += num;

        if ROOT {
            println!("{}: {num}", moves.list[m_idx].to_uci());
        }
    }

    positions
}