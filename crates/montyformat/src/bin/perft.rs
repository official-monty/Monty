use std::time::Instant;

use montyformat::chess::{perft, Castling, Position};

fn main() {
    let mut castling = Castling::default();
    let pos = Position::parse_fen(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &mut castling,
    );

    let now = Instant::now();
    let count = perft::<true>(&pos, &castling, 6);
    let time = now.elapsed().as_secs_f64();
    println!(
        "nodes {count} time {time:.3} nps {:.0}",
        count as f64 / time
    );
}
