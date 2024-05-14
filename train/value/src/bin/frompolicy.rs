use monty::ataxx::Ataxx;
use datagen::PolicyData;
use bullet::format::{AtaxxBoard, BulletFormat};

use std::{fs::File, io::BufWriter, time::Instant};

fn main() {
    let mut args = std::env::args();
    args.next();

    let data_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let mut raw_bytes = std::fs::read(data_path).unwrap();
    let data: &[PolicyData<Ataxx, 114>] = data_from_bytes_with_lifetime(&mut raw_bytes);

    let mut output =
        BufWriter::new(File::create(out_path.as_str()).expect("Provide a correct path!"));

    println!("# [Converting Data]");
    let time = Instant::now();

    let mut entries = Vec::new();
    for entry in data {
        entries.push(into_value(entry));

        if entries.len() % 16384 == 0 {
            AtaxxBoard::write_to_bin(&mut output, &entries).unwrap();
            entries.clear();
        }
    }

    AtaxxBoard::write_to_bin(&mut output, &entries).unwrap();

    println!("> Took {:.2} seconds.", time.elapsed().as_secs_f32());
}

fn into_value(pos: &PolicyData<Ataxx, 114>) -> AtaxxBoard {
    let board = pos.pos;
    let stm = board.stm();
    let bbs = board.bbs();

    let score = -(400.0 * (1.0 / pos.score - 1.0).ln()) as i16;

    AtaxxBoard::from_raw(bbs, score, pos.result, stm > 0, board.fullm(), board.halfm())
}

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
