use std::{fs::File, io::BufReader, sync::mpsc};

use bullet::{format::ChessBoard, loader::DataLoader};
use datagen::Rand;
use montyformat::{chess::Position, MontyValueFormat};

#[derive(Clone)]
pub struct BinpackLoader {
    file_path: [String; 1],
    buffer_size: usize,
}

impl BinpackLoader {
    pub fn new(path: &str, buffer_size_mb: usize) -> Self {
        Self {
            file_path: [path.to_string(); 1],
            buffer_size: buffer_size_mb * 1024 * 1024 / 80 / 2,
        }
    }
}

impl DataLoader<ChessBoard> for BinpackLoader {
    fn data_file_paths(&self) -> &[String] {
        &self.file_path
    }

    fn count_positions(&self) -> Option<u64> {
        None
    }

    fn map_batches<F: FnMut(&[ChessBoard]) -> bool>(&self, batch_size: usize, mut f: F) {
        let mut shuffle_buffer = Vec::new();
        shuffle_buffer.reserve_exact(self.buffer_size);

        let (buffer_sender, buffer_receiver) = mpsc::sync_channel::<Vec<(Position, f32, i16)>>(0);
        let (buffer_msg_sender, buffer_msg_receiver) = mpsc::sync_channel::<bool>(0);

        let file_path = self.file_path[0].clone();
        let buffer_size = self.buffer_size;

        std::thread::spawn(move || {
            let mut reusable_buffer = Vec::new();

            'dataloading: loop {
                let mut reader = BufReader::new(File::open(file_path.as_str()).unwrap());

                while let Ok(game) = MontyValueFormat::deserialise_from(&mut reader, Vec::new()) {
                    if buffer_msg_receiver.try_recv().unwrap_or(false) {
                        break 'dataloading;
                    }

                    parse_into_buffer(game, &mut reusable_buffer);

                    if shuffle_buffer.len() + reusable_buffer.len() < shuffle_buffer.capacity() {
                        shuffle_buffer.extend_from_slice(&reusable_buffer);
                    } else {
                        let diff = shuffle_buffer.capacity() - shuffle_buffer.len();
                        if diff > 0 {
                            shuffle_buffer.extend_from_slice(&reusable_buffer[..diff]);
                        }

                        shuffle(&mut shuffle_buffer);
                        
                        if buffer_msg_receiver.try_recv().unwrap_or(false) {
                            break 'dataloading;
                        } else {
                            buffer_sender.send(shuffle_buffer).unwrap();
                        }

                        shuffle_buffer = Vec::new();
                        shuffle_buffer.reserve_exact(buffer_size);
                    }
                }
            }
        });

        let (batch_sender, batch_reciever) = mpsc::sync_channel::<Vec<ChessBoard>>(16);
        let (batch_msg_sender, batch_msg_receiver) = mpsc::sync_channel::<bool>(0);

        std::thread::spawn(move || {
            let mut prealloc = Vec::new();
            prealloc.reserve_exact(batch_size);

            'dataloading: while let Ok(shuffle_buffer) = buffer_receiver.recv() {
                for batch in shuffle_buffer.chunks(batch_size) {
                    if batch_msg_receiver.try_recv().unwrap_or(false) {
                        buffer_msg_sender.send(true).unwrap();
                        break 'dataloading;
                    } else {
                        for (pos, result, score) in batch {
                            prealloc.push(
                                ChessBoard::from_raw(pos.bbs(), pos.stm(), *score, *result).unwrap()
                            );
                        }

                        batch_sender.send(prealloc).unwrap();

                        prealloc = Vec::new();
                        prealloc.reserve_exact(batch_size);
                    }
                }
            }
        });

        'dataloading: while let Ok(inputs) = batch_reciever.recv() {
            for batch in inputs.chunks(batch_size) {
                let should_break = f(batch);

                if should_break {
                    batch_msg_sender.send(true).unwrap();
                    break 'dataloading;
                }
            }
        }
    }
}

fn parse_into_buffer(game: MontyValueFormat, buffer: &mut Vec<(Position, f32, i16)>) {
    buffer.clear();

    let mut pos = game.startpos;
    let castling = game.castling;

    for data in game.moves {
        if data.score.abs() < 2000 && data.score != i16::MIN {
            buffer.push((pos, game.result, data.score));
        }

        pos.make(data.best_move, &castling);
    }
}

fn shuffle(data: &mut [(Position, f32, i16)]) {
    let mut rng = Rand::with_seed();

    for i in (0..data.len()).rev() {
        let idx = rng.rand_int() as usize % (i + 1);
        data.swap(idx, i);
    }
}
