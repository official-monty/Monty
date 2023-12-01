use monty_train::{data_from_bytes_with_lifetime, Rand, TrainingPosition, write_data};

use std::{fs::File, io::BufWriter, time::Instant};

fn shuffle(data: &mut [TrainingPosition]) {
    let mut rng = Rand::with_seed();

    for _ in 0..data.len() * 4 {
        let idx1 = rng.rand_int() as usize % data.len();
        let idx2 = rng.rand_int() as usize % data.len();
        data.swap(idx1, idx2);
    }
}

fn main() {
    let mut args = std::env::args();
    args.next();

    let data_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let mut raw_bytes = std::fs::read(data_path).unwrap();
    let data = data_from_bytes_with_lifetime(&mut raw_bytes);

    let mut output = BufWriter::new(File::create(out_path.as_str()).expect("Provide a correct path!"));

    println!("# [Shuffling Data]");
    let time = Instant::now();
    shuffle(data);
    println!("> Took {:.2} seconds.", time.elapsed().as_secs_f32());

    write_data(data, &mut output);
}