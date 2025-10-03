use acyclib::{
    device::tensor::Shape,
    trainer::{
        dataloader::{
            DataLoader, HostDenseMatrix, HostMatrix, HostSparseMatrix, PreparedBatchHost,
        },
        DataLoadingError,
    },
};
use monty::networks::policy::{
    inputs::map_features,
    outputs::{map_move_to_index, NUM_MOVES_INDICES},
    INPUT_SIZE,
};
use montyformat::chess::Move;

use super::reader::{DataReader, DecompressedData};
use crate::model::{MAX_ACTIVE_BASE, MAX_MOVES};

#[derive(Clone)]
pub struct MontyDataLoader {
    reader: DataReader,
    threads: usize,
}

impl MontyDataLoader {
    pub fn new(
        path: &str,
        buffer_size_mb: usize,
        reader_threads: usize,
        loader_threads: usize,
    ) -> Self {
        Self {
            reader: DataReader::new(path, buffer_size_mb, reader_threads),
            threads: loader_threads,
        }
    }
}

impl DataLoader for MontyDataLoader {
    type Error = DataLoadingError;

    fn map_batches<F: FnMut(PreparedBatchHost) -> bool>(
        self,
        batch_size: usize,
        mut f: F,
    ) -> Result<(), Self::Error> {
        self.reader
            .map_batches(batch_size, |batch| f(prepare(batch, self.threads)));

        Ok(())
    }
}

pub fn prepare(data: &[DecompressedData], threads: usize) -> PreparedBatchHost {
    let batch_size = data.len();
    let chunk_size = batch_size.div_ceil(threads);

    let mut inputs = vec![0; MAX_ACTIVE_BASE * batch_size];
    let mut moves = vec![0; MAX_MOVES * batch_size];
    let mut dist = vec![0.0; MAX_MOVES * batch_size];

    std::thread::scope(|s| {
        for (((data_chunk, input_chunk), moves_chunk), dist_chunk) in data
            .chunks(chunk_size)
            .zip(inputs.chunks_mut(MAX_ACTIVE_BASE * chunk_size))
            .zip(moves.chunks_mut(MAX_MOVES * chunk_size))
            .zip(dist.chunks_mut(MAX_MOVES * chunk_size))
        {
            s.spawn(move || {
                for (i, point) in data_chunk.iter().enumerate() {
                    let input_offset = MAX_ACTIVE_BASE * i;
                    let moves_offset = MAX_MOVES * i;

                    let mut j = 0;
                    map_features(&point.pos, |feat| {
                        assert!(feat < INPUT_SIZE);
                        input_chunk[input_offset + j] = feat as i32;
                        j += 1;
                    });

                    for k in j..MAX_ACTIVE_BASE {
                        input_chunk[input_offset + k] = -1;
                    }

                    assert!(
                        j <= MAX_ACTIVE_BASE,
                        "More inputs provided than the specified maximum!"
                    );

                    let mut total = 0;
                    let mut distinct = 0;

                    let pos = &point.pos;

                    for &(mov, visits) in &point.moves[..point.num] {
                        total += visits;

                        let mov = Move::from(mov);
                        moves_chunk[moves_offset + distinct] = map_move_to_index(pos, mov) as i32;
                        dist_chunk[moves_offset + distinct] = f32::from(visits);
                        distinct += 1;
                    }

                    for k in distinct..MAX_MOVES {
                        moves_chunk[moves_offset + k] = -1;
                    }

                    let total = f32::from(total);

                    for idx in 0..distinct {
                        dist_chunk[moves_offset + idx] /= total;
                    }
                }
            });
        }
    });

    let mut prep = PreparedBatchHost {
        batch_size,
        inputs: Default::default(),
    };

    unsafe {
        prep.inputs.insert(
            "inputs".to_string(),
            HostMatrix::Sparse(HostSparseMatrix::new(
                inputs,
                Some(batch_size),
                Shape::new(INPUT_SIZE, 1),
                MAX_ACTIVE_BASE,
            )),
        );

        prep.inputs.insert(
            "moves".to_string(),
            HostMatrix::Sparse(HostSparseMatrix::new(
                moves,
                Some(batch_size),
                Shape::new(NUM_MOVES_INDICES, 1),
                MAX_MOVES,
            )),
        );
    }

    prep.inputs.insert(
        "targets".to_string(),
        HostMatrix::Dense(HostDenseMatrix::new(
            dist,
            Some(batch_size),
            Shape::new(MAX_MOVES, 1),
        )),
    );

    prep
}
