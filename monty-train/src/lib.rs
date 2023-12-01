mod data;
mod datagen;
mod gradient;
mod rng;

pub use data::TrainingPosition;
pub use datagen::{DatagenThread, set_stop, write_data};
pub use gradient::gradient_batch;
pub use rng::Rand;


pub fn to_slice_with_lifetime<T, U>(slice: &[T]) -> &[U] {
    let src_size = std::mem::size_of_val(slice);
    let tgt_size = std::mem::size_of::<U>();

    assert!(
        src_size % tgt_size == 0,
        "Target type size does not divide slice size!"
    );

    let len = src_size / tgt_size;
    unsafe {
        std::slice::from_raw_parts(slice.as_ptr().cast(), len)
    }
}

pub fn data_from_bytes_with_lifetime(raw_bytes: &mut [u8]) -> &mut [TrainingPosition] {
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
