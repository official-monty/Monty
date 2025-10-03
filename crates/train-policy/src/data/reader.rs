use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

use montyformat::{
    chess::{Castling, Move, Position},
    FastDeserialise, MontyFormat,
};

use crate::model::MAX_MOVES;

#[derive(Clone, Copy)]
pub struct DecompressedData {
    pub pos: Position,
    pub castling: Castling,
    pub moves: [(u16, u16); MAX_MOVES],
    pub num: usize,
}

#[derive(Clone)]
pub struct DataReader {
    file_path: String,
    buffer_size: usize,
    threads: usize,
}

impl DataReader {
    pub fn new(path: &str, buffer_size_mb: usize, threads: usize) -> Self {
        Self {
            file_path: path.to_string(),
            buffer_size: buffer_size_mb * 1024 * 1024 / std::mem::size_of::<DecompressedData>() / 2,
            threads,
        }
    }
}

impl DataReader {
    pub fn map_batches<F: FnMut(&[DecompressedData]) -> bool>(&self, batch_size: usize, mut f: F) {
        let file_path = self.file_path.clone();
        let buffer_size = self.buffer_size;
        let threads = self.threads;
        let games_per_thread = 2048;

        let (game_sender, game_receiver) = mpsc::sync_channel::<Vec<u8>>(32);

        std::thread::spawn(move || {
            let mut buffer = Vec::new();

            'dataloading: loop {
                let mut reader = BufReader::new(File::open(file_path.as_str()).unwrap());

                while let Ok(()) =
                    MontyFormat::deserialise_fast_into_buffer(&mut reader, &mut buffer)
                {
                    if game_sender.send(buffer.clone()).is_err() {
                        break 'dataloading;
                    }
                }
            }
        });

        let (mini_sender, mini_receiver) = mpsc::sync_channel::<Vec<DecompressedData>>(threads);

        std::thread::spawn(move || {
            let mut buffer = Vec::new();

            while let Ok(game_bytes) = game_receiver.recv() {
                buffer.push(game_bytes);
                if buffer.len() == games_per_thread * threads {
                    if std::thread::scope(|s| {
                        let mut handles = Vec::with_capacity(threads);

                        for chunk in buffer.chunks(games_per_thread) {
                            let this_sender = mini_sender.clone();
                            let handle = s.spawn(move || {
                                let mut buf = Vec::with_capacity(160 * games_per_thread);

                                for game_bytes in chunk {
                                    parse_into_buffer(game_bytes, &mut buf);
                                }

                                this_sender.send(buf).is_err()
                            });

                            handles.push(handle);
                        }

                        handles.into_iter().any(|x| x.join().unwrap())
                    }) {
                        break;
                    }

                    buffer.clear();
                }
            }
        });

        let (buffer_sender, buffer_receiver) = mpsc::sync_channel::<Vec<DecompressedData>>(0);

        std::thread::spawn(move || {
            let mut shuffle_buffer = Vec::new();
            shuffle_buffer.reserve_exact(buffer_size);

            while let Ok(buffer) = mini_receiver.recv() {
                if shuffle_buffer.len() + buffer.len() < shuffle_buffer.capacity() {
                    shuffle_buffer.extend_from_slice(&buffer);
                } else {
                    let diff = shuffle_buffer.capacity() - shuffle_buffer.len();
                    shuffle_buffer.extend_from_slice(&buffer[..diff]);

                    if buffer_sender.send(shuffle_buffer).is_err() {
                        break;
                    }

                    shuffle_buffer = Vec::new();
                    shuffle_buffer.reserve_exact(buffer_size);
                }
            }
        });

        let (shuffled_sender, shuffled_receiver) = mpsc::sync_channel::<Vec<DecompressedData>>(0);

        std::thread::spawn(move || {
            while let Ok(mut inputs) = buffer_receiver.recv() {
                shuffle(&mut inputs);

                if shuffled_sender.send(inputs).is_err() {
                    break;
                }
            }
        });

        'dataloading: while let Ok(inputs) = shuffled_receiver.recv() {
            for batch in inputs.chunks(batch_size) {
                if f(batch) {
                    break 'dataloading;
                }
            }
        }

        drop(shuffled_receiver);
    }
}

fn shuffle(data: &mut [DecompressedData]) {
    let mut rng = Rand::with_seed();

    for i in (0..data.len()).rev() {
        let idx = rng.rng() as usize % (i + 1);
        data.swap(idx, i);
    }
}

macro_rules! read_into_primitive {
    ($reader:expr, $t:ty) => {{
        let mut buf = [0u8; std::mem::size_of::<$t>()];
        $reader.read_exact(&mut buf).unwrap();
        <$t>::from_le_bytes(buf)
    }};
}

fn parse_into_buffer(game: &[u8], buffer: &mut Vec<DecompressedData>) {
    let mut reader = Cursor::new(game);

    let mut qbbs = [0u64; 4];
    for bb in &mut qbbs {
        *bb = read_into_primitive!(reader, u64);
    }

    let stm = read_into_primitive!(reader, u8);
    let enp_sq = read_into_primitive!(reader, u8);
    let rights = read_into_primitive!(reader, u8);
    let halfm = read_into_primitive!(reader, u8);
    let fullm = read_into_primitive!(reader, u16);

    let mut bbs = [0; 8];

    let blc = qbbs[0];
    let rqk = qbbs[1];
    let nbk = qbbs[2];
    let pbq = qbbs[3];

    let occ = rqk | nbk | pbq;
    let pnb = occ ^ qbbs[1];
    let prq = occ ^ qbbs[2];
    let nrk = occ ^ qbbs[3];

    bbs[0] = occ ^ blc;
    bbs[1] = blc;
    bbs[2] = pnb & prq;
    bbs[3] = pnb & nrk;
    bbs[4] = pnb & nbk & pbq;
    bbs[5] = prq & nrk;
    bbs[6] = pbq & prq & rqk;
    bbs[7] = nbk & rqk;

    #[allow(deprecated)]
    let mut pos = Position::from_raw(bbs, stm > 0, enp_sq, rights, halfm, fullm);

    let mut rook_files = [[0; 2]; 2];
    for side in &mut rook_files {
        for rook in side {
            *rook = read_into_primitive!(reader, u8);
        }
    }

    let castling = Castling::from_raw(&pos, rook_files);

    let _result = read_into_primitive!(reader, u8) as f32 / 2.0;

    loop {
        let best_move = Move::from(read_into_primitive!(reader, u16));

        if best_move == Move::NULL {
            break;
        }

        let _score = f32::from(read_into_primitive!(reader, u16)) / f32::from(u16::MAX);

        let num_moves = usize::from(read_into_primitive!(reader, u8));

        if num_moves > 1 && num_moves <= MAX_MOVES {
            let mut policy_data = DecompressedData {
                pos,
                castling,
                moves: [(0, 0); MAX_MOVES],
                num: num_moves,
            };

            let mut count = 0;
            pos.map_legal_moves(&castling, |mov| {
                policy_data.moves[count].0 = mov.into();
                count += 1;
            });

            assert_eq!(count, num_moves);

            policy_data.moves[..num_moves].sort_by_key(|x| x.0);

            for entry in &mut policy_data.moves[..num_moves] {
                entry.1 = u16::from(read_into_primitive!(reader, u8));
            }

            buffer.push(policy_data);
        } else {
            for _ in 0..num_moves {
                let _ = read_into_primitive!(reader, u8);
            }
        }

        pos.make(best_move, &castling);
    }
}

pub struct Rand(u64);

impl Rand {
    pub fn with_seed() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Guaranteed increasing.")
            .as_micros() as u64
            & 0xFFFF_FFFF;

        Self(seed)
    }

    pub fn rng(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
