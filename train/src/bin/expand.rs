use std::{fs::File, io::BufWriter};

use bullet::format::{DataLoader, AtaxxBoard, BulletFormat};

fn main() {
    let mut args = std::env::args();
    args.next();

    let data_path = args.next().unwrap();

    transform(data_path.as_str(), "p1.data", |bb| flip_hor(flip_vert(bb)));

    transform(data_path.as_str(), "p2.data", flip_hor);

    transform(data_path.as_str(), "p3.data", flip_vert);

    transform(data_path.as_str(), "p4.data", flip_diag);

    transform(data_path.as_str(), "p5.data", |bb| flip_hor(flip_vert(flip_diag(bb))));
}

fn flip_vert(bb: u64) -> u64 {
    const RANK: u64 = 127;
    let mut out = 0;

    for rank in 0..7 {
        let iso = (bb >> (7 * rank)) & RANK;
        out |= iso << (7 * (6 - rank));
    }

    out
}

fn flip_hor(bb: u64) -> u64 {
    const FILE: u64 = 4432676798593;
    let mut out = 0;

    for file in 0..7 {
        let iso = (bb >> file) & FILE;
        out |= iso << (6 - file);
    }

    out
}

fn flip_diag(bb: u64) -> u64 {
    const RANK: u64 = 127;
    let mut out = 0;

    for rank in 0..7 {
        let mut iso = (bb >> (7 * rank)) & RANK;
        let mut file = 0;

        while iso > 0 {
            file |= 1 << (iso.trailing_zeros() * 7);
            iso &= iso - 1;
        }

        out |= file << rank;
    }

    out
}

fn transform<F: Fn(u64) -> u64>(data_path: &str, out_path: &str, f: F) {
    let loader = DataLoader::<AtaxxBoard>::new(data_path, 512).unwrap();

    let mut new = Vec::new();
    let mut total = 0;

    loader.map_positions(|pos| {
        total += 1;

        let mut bbs = pos.bbs();

        for bb in bbs.iter_mut() {
            *bb = f(*bb);
        }

        let stm = pos.stm() > 0;
        let halfm = pos.halfm();
        let fullm = pos.fullm();

        let mut score = pos.score();
        let mut result = pos.result();

        if stm {
            bbs.swap(0, 1);
            score = -score;
            result = 1.0 - result;
        }

        new.push(AtaxxBoard::from_raw(bbs, score, result, stm, fullm, halfm));

        if total % (16_384 * 32) == 0 {
            println!("Processed: {total}");
        }
    });

    let mut out = BufWriter::new(File::create(out_path).unwrap());
    println!("Writing to [{out_path}]");
    AtaxxBoard::write_to_bin(&mut out, &new).unwrap();
    println!("Processed: {total}");
}

#[cfg(test)]
fn display(bb: u64) {
    for rank in (0..7).rev() {
        for file in 0..7 {
            let sq = 7 * rank + file;
            let bit = 1 << sq;

            let add = if bit & bb > 0 {
                " x"
            } else {
                " ."
            };

            print!("{add}");
        }

        println!();
    }
}

#[test]
fn t() {
    let x = 14000221;
    display(x);
    println!();
    display(flip_vert(x));
    println!();
    display(flip_hor(x));
    println!();
    display(flip_hor(flip_vert(x)));
    println!();
    display(flip_diag(x));
    println!();
    display(flip_hor(flip_vert(flip_diag(x))));
}
