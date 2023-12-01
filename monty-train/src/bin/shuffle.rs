use monty_train::{Rand, TrainingPosition, write_data};

use std::{fs::File, io::BufWriter, time::Instant};

fn data_from_bytes_with_lifetime(raw_bytes: &mut [u8]) -> &mut [TrainingPosition] {
    let src_size = std::mem::size_of_val(raw_bytes);
    let tgt_size = std::mem::size_of::<TrainingPosition>();

    assert!(
        src_size % tgt_size == 0,
        "Target type size does not divide slice size!"
    );

    let len = src_size / tgt_size;
    unsafe {
        std::slice::from_raw_parts_mut(raw_bytes.as_mut_ptr().cast(), len)
    }
}

fn shuffle(data: &mut [TrainingPosition]) {
    let mut rng = Rand::with_seed();

    for _ in 0..data.len() * 16 {
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