use crate::Rand;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::sync::Arc;

const BOOK_CHECKPOINT_INTERVAL: usize = 1024;

#[derive(Clone)]
pub struct OpeningBook {
    data: Arc<OpeningBookData>,
}

struct OpeningBookData {
    path: String,
    checkpoints: Vec<u64>,
    line_count: usize,
}

impl OpeningBook {
    pub fn load(path: String) -> io::Result<Self> {
        let mut reader = BufReader::new(File::open(&path)?);

        let mut checkpoints = Vec::new();
        let mut line = Vec::new();
        let mut offset = 0u64;
        let mut line_count = 0usize;

        loop {
            line.clear();
            let bytes = reader.read_until(b'\n', &mut line)?;

            if bytes == 0 {
                break;
            }

            offset += bytes as u64;
            line_count += 1;

            if line_count % BOOK_CHECKPOINT_INTERVAL == 0 {
                checkpoints.push(offset);
            }
        }

        if line_count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "opening book contains no lines",
            ));
        }

        Ok(Self {
            data: Arc::new(OpeningBookData {
                path,
                checkpoints,
                line_count,
            }),
        })
    }

    pub fn reader(&self) -> io::Result<OpeningBookReader> {
        OpeningBookReader::new(self.data.clone())
    }
}

pub struct OpeningBookReader {
    book: Arc<OpeningBookData>,
    reader: BufReader<File>,
}

impl OpeningBookReader {
    fn new(book: Arc<OpeningBookData>) -> io::Result<Self> {
        let reader = BufReader::new(File::open(&book.path)?);

        Ok(Self { book, reader })
    }

    pub fn random_line(&mut self, rng: &mut Rand) -> io::Result<String> {
        let line_idx = (rng.rand_int() as usize) % self.book.line_count;

        let checkpoint_idx = line_idx / BOOK_CHECKPOINT_INTERVAL;
        let start_offset = if checkpoint_idx == 0 {
            0
        } else {
            self.book.checkpoints[checkpoint_idx - 1]
        };

        self.reader.seek(SeekFrom::Start(start_offset))?;

        let mut line = String::new();
        let lines_to_skip = line_idx % BOOK_CHECKPOINT_INTERVAL;

        for _ in 0..=lines_to_skip {
            line.clear();
            let bytes = self.reader.read_line(&mut line)?;

            if bytes == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unexpected end of file while reading opening book",
                ));
            }
        }

        while matches!(line.chars().last(), Some('\n' | '\r')) {
            line.pop();
        }

        Ok(line)
    }
}
