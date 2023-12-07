use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

use monty_train::{Rand, TrainingPosition};

const SIZE: usize = std::mem::size_of::<TrainingPosition>();

fn main() {
    let mut args = std::env::args();
    args.next();

    let collected: Vec<String> = args.collect();

    assert!(collected.len() > 1);

    let output = collected.last().unwrap();
    println!("Writing to {output}");
    let inputs: Vec<_> = collected.iter().take(collected.len() - 1).collect();
    println!("Reading from:\n{inputs:#?}");
    let mut streams = Vec::new();
    let mut total = 0;

    let target = File::create(output.as_str()).unwrap();
    let mut writer = BufWriter::new(target);

    for path in inputs {
        let file = File::open(path.as_str()).unwrap();
        let count = file.metadata().unwrap().len() as usize / SIZE;

        if count > 0 {
            streams.push((count, BufReader::new(file)));
            total += count;
        }
    }

    let mut remaining = total;
    let mut rng = Rand::with_seed();

    while remaining > 0 {
        let mut spot = rng.rand_int() as usize % remaining;
        let mut idx = 0;
        while streams[idx].0 < spot {
            spot -= streams[idx].0;
            idx += 1;
        }

        let (count, reader) = &mut streams[idx];
        let mut value = [0; SIZE];
        reader.read_exact(&mut value).unwrap();
        writer.write_all(&value).unwrap();

        remaining -= 1;
        *count -= 1;
        if *count == 0 {
            streams.swap_remove(idx);
        }

        if remaining % 16384 == 0 {
            let written = total - remaining;
            print!(
                "Written {written} / {total} ({:.2})\r",
                written as f32 / total as f32 * 100.0
            );
            let _ = std::io::stdout().flush();
        }
    }
}
