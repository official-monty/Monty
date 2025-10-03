use std::{
    fs::File,
    io::{BufReader, BufWriter, Cursor},
    sync::mpsc::{self, SyncSender},
    time::Instant,
};

use bullet_lib::game::formats::{
    bulletformat::{BulletFormat, ChessBoard},
    montyformat::{FastDeserialise, MontyValueFormat},
};

#[derive(Clone, Copy, Default)]
struct Stats {
    positions: usize,
    filtered: usize,
    checks: usize,
    caps: usize,
    scores: usize,
    games: usize,
}

fn main() {
    let mut args = std::env::args();
    args.next();

    let inp_path = args.next().unwrap();
    let out_path = args.next().unwrap();
    let threads = args.next().unwrap().parse().unwrap();
    let per_thread_batch_size = 8192 * 4;
    let batch_size = threads * per_thread_batch_size;

    let mut reader = BufReader::new(File::open(inp_path).unwrap());
    let mut writer = BufWriter::new(File::create(out_path).unwrap());

    let timer = Instant::now();

    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(256);

    std::thread::spawn(move || loop {
        let mut buffer = Vec::new();
        if let Ok(()) = MontyValueFormat::deserialise_fast_into_buffer(&mut reader, &mut buffer) {
            sender.send(buffer).unwrap();
        } else {
            break;
        }
    });

    let (sender2, receiver2) = mpsc::sync_channel::<Vec<ChessBoard>>(256);

    let lock = std::thread::spawn(move || {
        let mut stats = vec![Stats::default(); threads];
        let mut game_buffer = Vec::new();

        while let Ok(game_bytes) = receiver.recv() {
            game_buffer.push(game_bytes);
            if game_buffer.len() % batch_size == 0 {
                convert_buffer(threads, &sender2, &game_buffer, &mut stats);
                report(&stats, &timer);
                game_buffer.clear();
            }
        }

        if !game_buffer.is_empty() {
            convert_buffer(threads, &sender2, &game_buffer, &mut stats);
        }

        stats
    });

    while let Ok(buf) = receiver2.recv() {
        ChessBoard::write_to_bin(&mut writer, &buf).unwrap();
    }

    report(&lock.join().unwrap(), &timer);
}

fn convert_buffer(
    threads: usize,
    sender: &SyncSender<Vec<ChessBoard>>,
    games: &[Vec<u8>],
    stats: &mut [Stats],
) {
    let chunk_size = games.len().div_ceil(threads);

    std::thread::scope(|s| {
        for (chunk, sub_stats) in games.chunks(chunk_size).zip(stats.iter_mut()) {
            let this_sender = sender.clone();
            s.spawn(move || {
                for game_bytes in chunk {
                    convert(&this_sender, game_bytes, sub_stats);
                }
            });
        }
    });
}

fn convert(sender: &SyncSender<Vec<ChessBoard>>, game_bytes: &[u8], stats: &mut Stats) {
    let mut reader = Cursor::new(&game_bytes);
    let game = MontyValueFormat::deserialise_from(&mut reader, Vec::new()).unwrap();

    let mut buf = Vec::new();

    let mut pos = game.startpos;
    let castling = &game.castling;

    for result in game.moves {
        let mut write = true;

        if pos.in_check() {
            write = false;
            stats.checks += 1;
        }

        if result.best_move.is_capture() {
            write = false;
            stats.caps += 1;
        }

        if result.score == i16::MIN || result.score.abs() > 4000 {
            write = false;
            stats.scores += 1;
        }

        if write {
            buf.push(
                ChessBoard::from_raw(pos.bbs(), pos.stm(), result.score, game.result).unwrap(),
            );
        } else {
            stats.filtered += 1;
        }

        stats.positions += 1;
        pos.make(result.best_move, castling);
    }

    stats.games += 1;
    sender.send(buf).unwrap();
}

fn report(stats: &[Stats], timer: &Instant) {
    let mut positions = 0;
    let mut filtered = 0;
    let mut checks = 0;
    let mut caps = 0;
    let mut scores = 0;
    let mut games = 0;

    for sub_stats in stats {
        positions += sub_stats.positions;
        filtered += sub_stats.filtered;
        checks += sub_stats.checks;
        caps += sub_stats.caps;
        scores += sub_stats.scores;
        games += sub_stats.games;
    }

    println!("Positions: {positions}");
    println!("Games    : {games}");
    println!("Game Len : {:.2}", positions as f64 / games as f64);
    println!("Filtered : {filtered}");
    println!(" - Checks  : {checks}");
    println!(" - Captures: {caps}");
    println!(" - Scores  : {scores}");
    println!("Remaining: {}", positions - filtered);
    println!(
        "Speed: {:.0}k/sec",
        (positions / 1000) as f64 / timer.elapsed().as_secs_f64()
    );
    println!("---------------------");
}
