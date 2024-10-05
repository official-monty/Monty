use std::{fs::File, io::BufReader, sync::mpsc};

use datagen::{CompressedChessBoard, PolicyData, Rand};
use monty::Board;
use montyformat::MontyFormat;

pub struct DataLoader {
    file_path: String,
    buffer_size: usize,
    batch_size: usize,
}

impl DataLoader {
    pub fn new(path: &str, buffer_size_mb: usize, batch_size: usize) -> Self {
        Self {
            file_path: path.to_string(),
            buffer_size: buffer_size_mb * 1024 * 1024 / std::mem::size_of::<PolicyData>() / 2,
            batch_size,
        }
    }

    pub fn map_batches<F: FnMut(&[PolicyData]) -> bool>(&self, mut f: F) {
        let mut shuffle_buffer = Vec::new();
        shuffle_buffer.reserve_exact(self.buffer_size);

        let (buffer_sender, buffer_receiver) = mpsc::sync_channel::<Vec<PolicyData>>(0);
        let (buffer_msg_sender, buffer_msg_receiver) = mpsc::sync_channel::<bool>(0);

        let file_path = self.file_path.clone();
        let buffer_size = self.buffer_size;
        let batch_size = self.batch_size;

        std::thread::spawn(move || {
            let mut reusable_buffer = Vec::new();

            'dataloading: loop {
                let mut reader = BufReader::new(File::open(file_path.as_str()).unwrap());

                while let Ok(game) = MontyFormat::deserialise_from(&mut reader) {
                    if buffer_msg_receiver.try_recv().unwrap_or(false) {
                        break 'dataloading;
                    }

                    parse_into_buffer(game, &mut reusable_buffer);
    
                    if shuffle_buffer.len() + reusable_buffer.len() < shuffle_buffer.capacity() {
                        shuffle_buffer.extend_from_slice(&reusable_buffer);
                    } else {
                        let diff = shuffle_buffer.capacity() - shuffle_buffer.len();
                        shuffle_buffer.extend_from_slice(&reusable_buffer[..diff]);

                        shuffle(&mut shuffle_buffer);
    
                        if buffer_msg_receiver.try_recv().unwrap_or(false) {
                            break 'dataloading;
                        }

                        buffer_sender.send(shuffle_buffer).unwrap();

                        shuffle_buffer = Vec::new();
                        shuffle_buffer.reserve_exact(buffer_size);
                    }
                }
            }
        });

        'dataloading: while let Ok(inputs) = buffer_receiver.recv() {
            for batch in inputs.chunks(batch_size) {
                let should_break = f(batch);

                if should_break {
                    buffer_msg_sender.send(true).unwrap();
                    break 'dataloading;
                }
            }
        }
    }
}

fn shuffle(data: &mut [PolicyData]) {
    let mut rng = Rand::with_seed();

    for i in (0..data.len()).rev() {
        let idx = rng.rand_int() as usize % (i + 1);
        data.swap(idx, i);
    }
}

fn parse_into_buffer(game: MontyFormat, buffer: &mut Vec<PolicyData>) {
    buffer.clear();

    let mut pos = game.startpos;
    let castling = game.castling;

    for data in game.moves {
        if (data.score - 0.5).abs() > 0.49 {
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

                buffer.push(policy_data);
            }
        }

        pos.make(data.best_move, &castling);
    }
}
