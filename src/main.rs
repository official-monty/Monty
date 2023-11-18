mod attacks;
mod consts;
mod mcts;
mod moves;
mod params;
mod policy;
mod position;
mod uci;
mod value;

fn main() {
    // initialise engine
    let mut pos = position::Position::parse_fen(uci::STARTPOS);
    let mut params = params::TunableParams::default();
    let mut stack = Vec::new();
    let mut report_moves = false;

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
