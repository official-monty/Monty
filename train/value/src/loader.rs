use std::{fs::File, io::BufReader};

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

        'dataloading: loop {
            let mut reader = BufReader::new(File::open(self.file_path[0].as_str()).unwrap());

            loop {
                let err = Binpack::deserialise_map(&mut reader, |board, _mov, score, result| {
                    if !(should_break || score == i16::MIN || score.abs() > 2000) {
                        let position =
                            ChessBoard::from_raw(board.bbs(), board.stm(), score, result).unwrap();
                        shuffle_buffer.push(position);
                    }

                    if shuffle_buffer.len() == shuffle_buffer.capacity() {
                        shuffle(&mut shuffle_buffer);

                        for batch in shuffle_buffer.chunks(batch_size) {
                            should_break |= f(batch);
                        }

                        shuffle_buffer.clear();
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
    }
}

fn shuffle(data: &mut [ChessBoard]) {
    let mut rng = Rand::with_seed();

    for i in (0..data.len()).rev() {
        let idx = rng.rand_int() as usize % (i + 1);
        data.swap(idx, i);
    }
}
