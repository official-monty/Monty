use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
};

struct RandU64(u64);

impl RandU64 {
    fn rand(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub trait FastDeserialise {
    fn deserialise_fast_into_buffer(
        reader: &mut impl std::io::BufRead,
        buffer: &mut Vec<u8>,
    ) -> std::io::Result<()>;
}

pub fn interleave<T: FastDeserialise>(
    input_paths: &[String],
    output_path: &str,
    seed: u64,
) -> std::io::Result<()> {
    println!("Writing to {:#?}", output_path);
    println!("Reading from:\n{:#?}", input_paths);
    let mut streams = Vec::new();
    let mut total = 0;

    let target = File::create(output_path)?;
    let mut writer = BufWriter::new(target);

    for path in input_paths {
        let file = File::open(path)?;

        let count = file.metadata()?.len();

        if count > 0 {
            streams.push((count, BufReader::new(file)));
            total += count;
        }
    }

    let mut remaining = total;
    let mut rng = RandU64(seed);

    const INTERVAL: u64 = 1024 * 1024 * 256;
    let mut prev = remaining / INTERVAL;

    let mut buffer = Vec::new();

    while remaining > 0 {
        let mut spot = rng.rand() % remaining;
        let mut idx = 0;
        while streams[idx].0 < spot {
            spot -= streams[idx].0;
            idx += 1;
        }

        let (count, reader) = &mut streams[idx];

        T::deserialise_fast_into_buffer(reader, &mut buffer)?;
        writer.write_all(&buffer)?;

        let size = buffer.len() as u64;

        remaining -= size;
        *count -= size;
        if *count == 0 {
            streams.swap_remove(idx);
        }

        if remaining / INTERVAL < prev {
            prev = remaining / INTERVAL;
            let written = total - remaining;
            print!(
                "Written {written}/{total} Bytes ({:.2}%)\r",
                written as f64 / total as f64 * 100.0
            );
            let _ = std::io::stdout().flush();
        }
    }

    Ok(())
}
