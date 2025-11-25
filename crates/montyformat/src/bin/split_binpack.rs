use std::{
    env,
    fs::File,
    io::{self, BufReader, BufWriter, Error, ErrorKind, Write},
    path::{Path, PathBuf},
};

use montyformat::{FastDeserialise, MontyFormat, MontyValueFormat};

const PROGRESS_INTERVAL: u64 = 1024 * 1024 * 256;

struct Progress {
    total: u64,
    remaining: u64,
    prev_interval: u64,
}

impl Progress {
    fn new(total: u64) -> Self {
        let remaining = total;
        let prev_interval = remaining / PROGRESS_INTERVAL;

        Self {
            total,
            remaining,
            prev_interval,
        }
    }

    fn update(&mut self, bytes_written: u64) {
        if bytes_written == 0 || self.total == 0 {
            return;
        }

        self.remaining = self.remaining.saturating_sub(bytes_written);

        if self.remaining / PROGRESS_INTERVAL < self.prev_interval {
            self.prev_interval = self.remaining / PROGRESS_INTERVAL;
            let written = self.total - self.remaining;
            print!(
                "Written {written}/{total} Bytes ({:.2}%)\r",
                written as f64 / self.total as f64 * 100.0,
                total = self.total
            );
            let _ = io::stdout().flush();
        }
    }
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

    let total_games = count_games::<T>(input_path)?;
    let total_bytes = File::open(input_path)?.metadata()?.len();

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

    let mut reader = BufReader::new(File::open(input_path)?);
    let mut buffer = Vec::new();
    let mut progress = Progress::new(total_bytes);

    for (idx, &games_in_part) in games_per_part.iter().enumerate() {
        let part_index = idx + 1;
        let output_path = numbered_output_path(input_path, part_index);
        println!("Writing {games_in_part} games to {}", output_path.display());
        let mut writer = BufWriter::new(File::create(output_path)?);

        for _ in 0..games_in_part {
            read_game_into_buffer::<T>(&mut reader, &mut buffer)?;
            writer.write_all(&buffer)?;
            progress.update(buffer.len() as u64);
        }
        writer.flush()?;
    }

    Ok(())
}

fn split_by_games<T: FastDeserialise>(input_path: &Path, games_per_file: usize) -> io::Result<()> {
    if games_per_file == 0 {
        return Err(usage_error("Games per file must be greater than zero"));
    }

    let mut reader = BufReader::new(File::open(input_path)?);
    let mut buffer = Vec::new();
    let mut current_writer: Option<BufWriter<File>> = None;
    let mut games_written_in_current = 0usize;
    let mut file_index = 0usize;
    let total_bytes = reader.get_ref().metadata()?.len();
    let mut progress = Progress::new(total_bytes);

    loop {
        match T::deserialise_fast_into_buffer(&mut reader, &mut buffer) {
            Ok(()) => {
                if current_writer.is_none() {
                    file_index += 1;
                    let output_path = numbered_output_path(input_path, file_index);
                    println!("Starting new file {}", output_path.display());
                    current_writer = Some(BufWriter::new(File::create(output_path)?));
                    games_written_in_current = 0;
                }

                if let Some(writer) = current_writer.as_mut() {
                    writer.write_all(&buffer)?;
                    games_written_in_current += 1;
                    progress.update(buffer.len() as u64);
                }

                if games_written_in_current == games_per_file {
                    if let Some(mut writer) = current_writer.take() {
                        writer.flush()?;
                    }
                }
            }
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                if let Some(mut writer) = current_writer.take() {
                    if games_written_in_current > 0 {
                        writer.flush()?;
                    }
                }
                break;
            }
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

fn read_game_into_buffer<T: FastDeserialise>(
    reader: &mut BufReader<File>,
    buffer: &mut Vec<u8>,
) -> io::Result<()> {
    T::deserialise_fast_into_buffer(reader, buffer).or_else(|err| {
        if err.kind() == ErrorKind::UnexpectedEof {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Encountered EOF while reading expected game",
            ))
        } else {
            Err(err)
        }
    })
}

fn count_games<T: FastDeserialise>(input_path: &Path) -> io::Result<usize> {
    let mut reader = BufReader::new(File::open(input_path)?);
    let mut buffer = Vec::new();
    let mut count = 0usize;

    loop {
        match T::deserialise_fast_into_buffer(&mut reader, &mut buffer) {
            Ok(()) => count += 1,
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(err),
        }
    }

    Ok(count)
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
