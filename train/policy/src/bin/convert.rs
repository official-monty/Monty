use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use monty::Board;
use montyformat::MontyFormat;

use datagen::{write, CompressedChessBoard, PolicyData};

fn main() {
    let mut args = std::env::args();
    args.next();

    let inp_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let mut reader = BufReader::new(File::open(inp_path).unwrap());
    let mut writer = BufWriter::new(File::create(out_path).unwrap());

    let mut positions = 0;
    let mut filtered = 0;
    let mut scores = 0;
    let mut games = 0;

    while let Ok(game) = MontyFormat::deserialise_from(&mut reader) {
        let mut pos = game.startpos;
        let castling = game.castling;

        for data in game.moves {
            if (data.score - 0.5).abs() > 0.49 {
                filtered += 1;
                scores += 1;
            } else if let Some(dist) = data.visit_distribution.as_ref() {
                if dist.len() < 112 {
                    let board = Board::from_raw(
                        pos.bbs(),
                        pos.stm() > 0,
                        pos.enp_sq(),
                        pos.rights(),
                        pos.halfm(),
                        pos.fullm(),
                    );

                    let mut policy_data = PolicyData {
                        pos: CompressedChessBoard::from(board),
                        moves: [(0, 0); 112],
                        num: dist.len(),
                        score: data.score,
                        result: game.result,
                        best_move: u16::from(data.best_move),
                    };

                    for (i, (mov, visits)) in dist.iter().enumerate() {
                        policy_data.moves[i] = (u16::from(*mov), *visits as u16);
                    }

                    positions += 1;

                    if positions % 4194304 == 0 {
                        println!("Processed: {positions}");
                    }

                    write(&[policy_data], &mut writer);
                }
            }

            pos.make(data.best_move, &castling);
        }

        games += 1;
    }

    println!("Positions: {positions}");
    println!("Games    : {games}");
    println!("Game Len : {:.2}", positions as f64 / games as f64);
    println!("Filtered : {filtered}");
    println!(" - Scores  : {scores}");
    println!("Remaining: {}", positions - filtered);
}
