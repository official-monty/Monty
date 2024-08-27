use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
};

use montyformat::MontyFormat;

fn main() -> std::io::Result<()> {
    let inputs = [
        "../binpacks/policygen4.binpack",
        "../binpacks/policygen4-dfrc.binpack",
        "../binpacks/policygen6.binpack",
    ];
    let output = "interleaved.binpack";

    println!("Writing to {:#?}", output);
    println!("Reading from:\n{:#?}", inputs);
    let mut streams = Vec::new();
    let mut total = 0;

    let target = File::create(output)?;
    let mut writer = BufWriter::new(target);

    for path in &inputs {
        let file = File::open(path)?;

        let count = file.metadata()?.len();

        if count > 0 {
            streams.push((count, BufReader::new(file)));
            total += count;
        }
    }

    let mut remaining = total;
    let mut rng = RandU64::default();

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

        MontyFormat::deserialise_fast_into_buffer(reader, &mut buffer)?;
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

struct RandU64(u64);

impl Default for RandU64 {
    fn default() -> Self {
        Self(
            (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("valid")
                .as_nanos()
                & 0xFFFF_FFFF_FFFF_FFFF) as u64,
        )
    }
}

impl RandU64 {
    fn rand(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
