use std::{fs::File, io::BufWriter};

use bullet::format::{DataLoader, AtaxxBoard, BulletFormat};

fn main() {
    let mut args = std::env::args();
    args.next();

    let data_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let loader = DataLoader::<AtaxxBoard>::new(data_path, 512).unwrap();

    let mut new = Vec::new();
    let mut total = 0;
    let mut filtered = 0;

    loader.map_positions(|pos| {
        total += 1;
        if pos.score().abs() < 5000 {
            new.push(*pos);
        } else {
            filtered += 1;
        }

        if total % (16_384 * 32) == 0 {
            println!("Processed: {total}, Filtered: {filtered}");
        }
    });

    let mut out = BufWriter::new(File::create(out_path.as_str()).unwrap());
    println!("Writing to [{out_path}]");
    AtaxxBoard::write_to_bin(&mut out, &new).unwrap();
    println!("Processed: {total}, Filtered: {filtered}");
}