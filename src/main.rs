#![warn(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

mod attacks;
mod consts;
mod position;
mod uci;

fn main() {
    // initialise engine
    let mut pos = position::Position::parse_fen(uci::STARTPOS);

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
            "position" => uci::position(commands, &mut pos, &mut Vec::new()),
            "perft" => uci::perft(&commands, &pos),
            "quit" => std::process::exit(0),
            _ => {}
        }
    }
}


