use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
};

fn main() {
    let mut args = std::env::args();
    args.next();

    let inp_path = args.next().unwrap();
    let out_path = args.next().unwrap();

    let mut reader = BufReader::new(File::open(inp_path).unwrap());
    let mut writer = BufWriter::new(File::create(out_path).unwrap());

    let mut positions = 0usize;
    let mut games = 0usize;

    while let Ok(pos) = convert_game(&mut reader, &mut writer) {
        games += 1;
        positions += pos;

        if games % (16384 * 8) == 0 {
            println!("Converted {games} Games, {positions} Positions");
        }
    }

    println!("Converted {games} Games, {positions} Positions")
}

fn convert_game(reader: &mut BufReader<File>, writer: &mut BufWriter<File>) -> io::Result<usize> {
    let mut buf = [0u8; 36];
    reader.read_exact(&mut buf)?;
    writer.write_all(&buf).unwrap();
    writer.write_all(&[0; 2]).unwrap();

    let mut buf = [0u8; 5];
    reader.read_exact(&mut buf)?;
    writer.write_all(&buf).unwrap();

    let mut positions = 0;

    loop {
        positions += 1;

        let mut buf = [0; 4];
        reader.read_exact(&mut buf).unwrap();
        writer.write_all(&buf).unwrap();

        if buf == [0; 4] {
            break;
        }
    }

    Ok(positions)
}
