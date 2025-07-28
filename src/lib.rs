pub mod chess;
pub mod mcts;
pub mod networks;
pub mod tree;
pub mod uci;
pub mod correction_history;

use memmap2::Mmap;

pub struct MappedWeights<'a, T> {
    pub mmap: Mmap,  // The memory-mapped file
    pub data: &'a T, // A reference to the data in the mmap
}

#[macro_export]
macro_rules! init {
    (|$sq:ident, $size:literal | $($rest:tt)+) => {{
        let mut $sq = 0;
        let mut res = [{$($rest)+}; $size];
        while $sq < $size {
            res[$sq] = {$($rest)+};
            $sq += 1;
        }
        res
    }};
}

#[macro_export]
macro_rules! init_add_assign {
    (|$sq:ident, $init:expr, $size:literal | $($rest:tt)+) => {{
        let mut $sq = 0;
        let mut res = [{$($rest)+}; $size + 1];
        let mut val = $init;
        while $sq < $size {
            res[$sq] = val;
            val += {$($rest)+};
            $sq += 1;
        }

        res[$size] = val;

        res
    }};
}

#[macro_export]
macro_rules! pop_lsb {
    ($idx:ident, $x:expr) => {
        let $idx = $x.trailing_zeros() as u16;
        $x &= $x - 1
    };
}

/// # Safety
/// Object must be valid if fully zeroed.
pub unsafe fn boxed_and_zeroed<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(ptr.cast())
    }
}

/// # Safety
/// Only to be used internally.
pub unsafe fn read_into_struct_unchecked<'a, T>(path: &str) -> MappedWeights<'a, T> {
    let f = std::fs::File::open(path).unwrap();
    let mmap = Mmap::map(&f).unwrap();

    let size = std::mem::size_of::<T>();
    let file_size = mmap.len();
    assert_eq!(
        file_size, size,
        "File size does not match the size of the structure"
    );

    let ptr = mmap.as_ptr() as *const T;

    // Check if the pointer is properly aligned
    if (ptr as usize) % std::mem::align_of::<T>() != 0 {
        panic!("Memory is not properly aligned for the type");
    }

    MappedWeights {
        mmap, // This ensures the memory is valid as long as MappedWeights exists
        data: &*ptr,
    }
}
