use datagen::{write, Rand};

use std::{fs::File, io::BufWriter, time::Instant};

const SIZE: usize = 512;

fn data_from_bytes_with_lifetime<T>(raw_bytes: &mut [u8]) -> &mut [T] {
    let src_size = std::mem::size_of_val(raw_bytes);
    let tgt_size = std::mem::size_of::<T>();

    assert!(
        src_size % tgt_size == 0,
        "Target type size does not divide slice size!"
    );

    let len = src_size / tgt_size;
    unsafe { std::slice::from_raw_parts_mut(raw_bytes.as_mut_ptr().cast(), len) }
}

fn shuffle(data: &mut [[u8; SIZE]]) {
    let mut rng = Rand::with_seed();

    for i in (0..data.len()).rev() {
        let idx = rng.rand_int() as usize % (i + 1);
        data.swap(idx, i);
    }
}

fn main() {
    let mut args = std::env::args();
    args.next();

    let data_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let mut raw_bytes = std::fs::read(data_path).unwrap();
    let data = data_from_bytes_with_lifetime(&mut raw_bytes);

    let mut output =
        BufWriter::new(File::create(out_path.as_str()).expect("Provide a correct path!"));

    println!("# [Shuffling Data]");
    let time = Instant::now();
    shuffle(data);
    println!("> Took {:.2} seconds.", time.elapsed().as_secs_f32());

    write(data, &mut output);
}
