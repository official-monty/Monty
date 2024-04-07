use std::{fs::File, io::BufWriter};

use bullet::format::{BulletFormat, DataLoader};

fn main() {
    filter::<bullet::format::ChessBoard>();
}

fn filter<T: BulletFormat>() {
    let mut args = std::env::args();
    args.next();

    let data_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let loader = DataLoader::<T>::new(data_path, 512).unwrap();

    let mut new = Vec::new();
    let mut total = 0;
    let mut filtered = 0;

    loader.map_positions(|pos| {
        total += 1;
        let raw_score = pos.score();
        let score = 1.0 / (1.0 + (-f32::from(raw_score) / 400.0).exp());
        let result = pos.result();

        let err = (score - result).abs();

        if err < 0.7 && raw_score.abs() < 1500 {
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
    T::write_to_bin(&mut out, &new).unwrap();
    println!("Processed: {total}, Filtered: {filtered}");
}
