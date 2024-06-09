use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

type T = datagen::PolicyData;
const S: usize = std::mem::size_of::<T>();

fn main() {
    let mut args = std::env::args();
    args.next();

    let inp_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let target = File::create(out_path).unwrap();
    let mut writer = BufWriter::new(target);

    let file = File::open(inp_path).unwrap();
    let count = file.metadata().unwrap().len() as usize / std::mem::size_of::<T>();
    let mut reader = BufReader::new(file);
    let mut remaining = count;
    let mut filtered = 0;

    while remaining > 0 {
        remaining -= 1;
        let mut value = [0; S];
        reader.read_exact(&mut value).unwrap();

        let board: T = unsafe { std::mem::transmute(value) };

        if (board.score - 0.5).abs() > 0.49 {
            filtered += 1;
            continue;
        }

        writer.write_all(&value).unwrap();

        if remaining % 16384 * 32 == 0 {
            let written = count - remaining;
            print!(
                "Processed {written} / {count} ({:.2}), Filtered {filtered}\r",
                written as f32 / count as f32 * 100.0
            );
            let _ = std::io::stdout().flush();
        }
    }

    println!();
}
