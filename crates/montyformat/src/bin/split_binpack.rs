use std::{
    env,
    fs::File,
    io::{self, BufWriter, Error, ErrorKind, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};

use memmap2::MmapOptions;
use montyformat::{FastDeserialise, MontyFormat, MontyValueFormat};
use rayon::prelude::*;

const PROGRESS_INTERVAL: u64 = 1024 * 1024 * 256;

#[derive(Clone)]
struct Progress {
    total: u64,
    written: Arc<AtomicU64>,
    next_interval: Arc<AtomicU64>,
}

impl Progress {
    fn new(total: u64) -> Self {
        Self {
            total,
            written: Arc::new(AtomicU64::new(0)),
            next_interval: Arc::new(AtomicU64::new(PROGRESS_INTERVAL)),
        }
    }

    fn update(&self, bytes: u64) {
        if bytes == 0 || self.total == 0 {
            return;
        }

        let written = self.written.fetch_add(bytes, Ordering::Relaxed) + bytes;

        loop {
            let next = self.next_interval.load(Ordering::Relaxed);
            if written < next && written < self.total {
                break;
            }

            if self
                .next_interval
                .compare_exchange(
                    next,
                    next.saturating_add(PROGRESS_INTERVAL),
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                let capped_written = written.min(self.total);
                print!(
                    "Written {capped_written}/{total} Bytes ({:.2}%)\r",
                    capped_written as f64 / self.total as f64 * 100.0,
                    total = self.total
                );
                let _ = io::stdout().flush();
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GameSpan {
    start: usize,
    len: usize,
}

struct ScanResult {
    spans: Vec<GameSpan>,
    mmap: Arc<memmap2::Mmap>,
    total_bytes: u64,
}

fn scan_games<T: FastDeserialise>(input_path: &Path) -> io::Result<ScanResult> {
    let file = File::open(input_path)?;
    let mmap = Arc::new(unsafe { MmapOptions::new().map(&file)? });
    let total_bytes = mmap.len() as u64;
    let progress = Progress::new(total_bytes);

    let cursor = Arc::new(AtomicUsize::new(0));
    let spans = Arc::new(Mutex::new(Vec::new()));
    let error: Arc<Mutex<Option<io::Error>>> = Arc::new(Mutex::new(None));
    let threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    thread::scope(|scope| {
        for _ in 0..threads {
            let cursor = Arc::clone(&cursor);
            let spans = Arc::clone(&spans);
            let mmap = Arc::clone(&mmap);
            let error = Arc::clone(&error);
            let progress = progress.clone();

            scope.spawn(move || {
                let mut buffer = Vec::new();

                loop {
                    let start = cursor.load(Ordering::Relaxed);
                    if start >= mmap.len() {
                        break;
                    }

                    let mut reader = io::Cursor::new(&mmap[start..]);
                    match T::deserialise_fast_into_buffer(&mut reader, &mut buffer) {
                        Ok(()) => {
                            let consumed = reader.position() as usize;
                            if consumed == 0 {
                                break;
                            }

                            if cursor
                                .compare_exchange(
                                    start,
                                    start + consumed,
                                    Ordering::SeqCst,
                                    Ordering::Relaxed,
                                )
                                .is_err()
                            {
                                continue;
                            }

                            progress.update(consumed as u64);
                            spans.lock().unwrap().push(GameSpan {
                                start,
                                len: consumed,
                            });
                        }
                        Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                            cursor.store(mmap.len(), Ordering::SeqCst);
                            break;
                        }
                        Err(err) => {
                            cursor.store(mmap.len(), Ordering::SeqCst);
                            *error.lock().unwrap() = Some(err);
                            break;
                        }
                    }
                }
            });
        }
    });

    if let Some(err) = error.lock().unwrap().take() {
        return Err(err);
    }

    let mut spans = Arc::try_unwrap(spans).unwrap().into_inner().unwrap();
    spans.sort_by_key(|span| span.start);

    Ok(ScanResult {
        spans,
        mmap: Arc::clone(&mmap),
        total_bytes,
    })
}

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);

    let format_kind = args
        .next()
        .ok_or_else(|| usage_error("Missing format kind (policy/value)"))?;

    let input_path = args
        .next()
        .ok_or_else(|| usage_error("Missing input binpack path"))?;

    let mode_flag = args
        .next()
        .ok_or_else(|| usage_error("Missing split mode (--parts or --games-per-file)"))?;

    let value = args
        .next()
        .ok_or_else(|| usage_error("Missing value for split mode"))?;

    if args.next().is_some() {
        return Err(usage_error("Too many arguments"));
    }

    let input_path = Path::new(&input_path);

    match format_kind.as_str() {
        "policy" => match mode_flag.as_str() {
            "--parts" | "-p" => {
                let parts = parse_positive_usize(&value, "parts")?;
                split_into_parts::<MontyFormat>(input_path, parts)
            }
            "--games-per-file" | "-g" => {
                let games = parse_positive_usize(&value, "games per file")?;
                split_by_games::<MontyFormat>(input_path, games)
            }
            _ => Err(usage_error("Unknown mode flag")),
        },
        "value" => match mode_flag.as_str() {
            "--parts" | "-p" => {
                let parts = parse_positive_usize(&value, "parts")?;
                split_into_parts::<MontyValueFormat>(input_path, parts)
            }
            "--games-per-file" | "-g" => {
                let games = parse_positive_usize(&value, "games per file")?;
                split_by_games::<MontyValueFormat>(input_path, games)
            }
            _ => Err(usage_error("Unknown mode flag")),
        },
        _ => Err(usage_error(
            "Unknown format kind. Expected 'policy' or 'value'",
        )),
    }
}

fn parse_positive_usize(value: &str, name: &str) -> io::Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| usage_error(&format!("Invalid {name}: {value}")))
        .and_then(|parsed| {
            if parsed == 0 {
                Err(usage_error(&format!("{name} must be greater than zero")))
            } else {
                Ok(parsed)
            }
        })
}

fn split_into_parts<T: FastDeserialise>(input_path: &Path, parts: usize) -> io::Result<()> {
    if parts == 0 {
        return Err(usage_error("Number of parts must be greater than zero"));
    }

    let scan = scan_games::<T>(input_path)?;
    let total_games = scan.spans.len();
    let total_bytes = scan.total_bytes;

    if total_games == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input binpack does not contain any games",
        ));
    }

    if total_games < parts {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Requested {parts} parts but only {total_games} games are available"),
        ));
    }

    let base = total_games / parts;
    let remainder = total_games % parts;

    let mut games_per_part = vec![base; parts];
    for chunk in games_per_part.iter_mut().take(remainder) {
        *chunk += 1;
    }

    let spans = Arc::new(scan.spans);
    let mmap = Arc::clone(&scan.mmap);
    let progress = Progress::new(total_bytes);

    let mut start_idx = 0usize;
    let part_specs: Vec<_> = games_per_part
        .iter()
        .enumerate()
        .map(|(idx, &games_in_part)| {
            let end_idx = start_idx + games_in_part;
            let spec = (idx + 1, start_idx, end_idx);
            start_idx = end_idx;
            spec
        })
        .collect();

    part_specs.into_par_iter().try_for_each(
        |(part_index, start_idx, end_idx)| -> io::Result<()> {
            let output_path = numbered_output_path(input_path, part_index);
            println!(
                "Writing {games_in_part} games to {}",
                output_path.display(),
                games_in_part = end_idx - start_idx
            );

            let mut writer = BufWriter::new(File::create(output_path)?);

            for span in &spans[start_idx..end_idx] {
                writer.write_all(&mmap[span.start..span.start + span.len])?;
                progress.update(span.len as u64);
            }

            writer.flush()
        },
    )?;

    Ok(())
}

fn split_by_games<T: FastDeserialise>(input_path: &Path, games_per_file: usize) -> io::Result<()> {
    if games_per_file == 0 {
        return Err(usage_error("Games per file must be greater than zero"));
    }

    let scan = scan_games::<T>(input_path)?;
    let spans = Arc::new(scan.spans);
    let mmap = Arc::clone(&scan.mmap);
    let progress = Progress::new(scan.total_bytes);

    let file_specs: Vec<(usize, usize, usize)> = spans
        .chunks(games_per_file)
        .enumerate()
        .map(|(idx, chunk)| (idx + 1, idx * games_per_file, chunk.len()))
        .collect();

    file_specs.into_par_iter().try_for_each(
        |(file_index, start_idx, count)| -> io::Result<()> {
            let end_idx = start_idx + count;
            let output_path = numbered_output_path(input_path, file_index);
            println!("Starting new file {}", output_path.display());

            let mut writer = BufWriter::new(File::create(output_path)?);
            for span in &spans[start_idx..end_idx] {
                writer.write_all(&mmap[span.start..span.start + span.len])?;
                progress.update(span.len as u64);
            }
            writer.flush()
        },
    )?;

    Ok(())
}

fn numbered_output_path(input_path: &Path, index: usize) -> PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("split");
    parent.join(format!("{stem}_{index:03}.binpack"))
}

fn usage_error(message: &str) -> io::Error {
    Error::new(ErrorKind::InvalidInput, format!(
        "{message}. Usage: split_binpack <policy|value> <input.binpack> (--parts <count> | --games-per-file <count>)"
    ))
}
