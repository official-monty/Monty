use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

use montyformat::{MontyFormat, MontyValueFormat};

fn main() {
    let mut args = std::env::args();
    args.next();

    let inp_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let mut reader = BufReader::new(File::open(inp_path).unwrap());
    let mut writer = BufWriter::new(File::create(out_path).unwrap());

    let mut positions = 0;
    let mut games = 0;

    let mut moves = Vec::new();

    while let Ok(game) = MontyFormat::deserialise_from(&mut reader) {
        moves.clear();

        let mut value = MontyValueFormat {
            startpos: game.startpos,
            castling: game.castling,
            result: game.result,
            moves,
        };

        let mut stm = value.startpos.stm();

        for result in game.moves {
            positions += 1;
            value.push(stm, result.best_move, result.score);
            stm = 1 - stm;
        }

        MontyValueFormat::serialise_into(&value, &mut writer).unwrap();

        games += 1;
        moves = value.moves;

        if games % 16384 == 0 {
            println!("Converted {games} games.");
        }
    }

    println!("Positions    : {positions}");
    println!("Games        : {games}");
    println!("Avg Game Len : {:.2}", positions as f64 / games as f64);
}
