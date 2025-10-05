use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
};

use montyformat::{FastDeserialise, MontyFormat};

fn main() {
    let mut reader = BufReader::new(File::open("../binpacks/policygen6.binpack").unwrap());
    let mut writer = BufWriter::new(File::create("a.binpack").unwrap());

    let mut reusable_buffer = Vec::new();

    while let Ok(()) = MontyFormat::deserialise_fast_into_buffer(&mut reader, &mut reusable_buffer)
    {
        writer.write_all(&reusable_buffer).unwrap();
    }
}
