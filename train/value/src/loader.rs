use std::{fs::File, io::BufReader, sync::mpsc};

use bullet::{format::ChessBoard, loader::DataLoader};
use datagen::{Binpack, Rand};

#[derive(Clone)]
pub struct BinpackLoader {
    file_path: [String; 1],
    buffer_size: usize,
}

impl BinpackLoader {
    pub fn new(path: &str, buffer_size_mb: usize) -> Self {
        Self {
            file_path: [path.to_string(); 1],
            buffer_size: buffer_size_mb * 1024 * 1024 / std::mem::size_of::<ChessBoard>() / 2,
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

        let mut should_break = false;

        let (batch_sender, batch_reciever) = mpsc::sync_channel::<Vec<ChessBoard>>(1);
        let (batch_msg_sender, batch_msg_receiver) = mpsc::sync_channel::<bool>(0);

        let file_path = self.file_path[0].clone();

        std::thread::spawn(move || {
            'dataloading: loop {
                let mut reader = BufReader::new(File::open(file_path.as_str()).unwrap());

                loop {
                    if batch_msg_receiver.try_recv().unwrap_or(false) {
                        should_break = true;
                    }                    

                    let err = Binpack::deserialise_map(&mut reader, |board, _mov, score, result| {
                        if !(should_break || score == i16::MIN || score.abs() > 2000) {
                            let position =
                                ChessBoard::from_raw(board.bbs(), board.stm(), score, result).unwrap();
                            shuffle_buffer.push(position);
                        }

                        if shuffle_buffer.len() % 1000000 == 0 {
                            println!("{}", shuffle_buffer.len());
                        }

                        if shuffle_buffer.len() == shuffle_buffer.capacity() {
                            if batch_msg_receiver.try_recv().unwrap_or(false) {
                                should_break = true;
                            } else if !should_break {
                                shuffle(&mut shuffle_buffer);
                                batch_sender.send(shuffle_buffer.clone()).unwrap();
                                shuffle_buffer.clear();
                            }
                        }
                    });

                    if should_break {
                        break 'dataloading;
                    }

                    if err.is_err() {
                        break;
                    }
                }
            }
        });

        while let Ok(inputs) = batch_reciever.recv() {
            for batch in inputs.chunks(batch_size) {
                let should_break = f(batch);

                if should_break {
                    batch_msg_sender.send(true).unwrap();
                    break;
                }
            }
        }
    }
}

fn shuffle(data: &mut [ChessBoard]) {
    let mut rng = Rand::with_seed();

    for i in (0..data.len()).rev() {
        let idx = rng.rand_int() as usize % (i + 1);
        data.swap(idx, i);
    }
}
