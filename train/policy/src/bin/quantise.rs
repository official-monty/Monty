use std::io::Write;

use monty::{read_into_struct_unchecked, UnquantisedPolicyNetwork, PolicyNetwork};

fn main() {
    let unquantised: Box<UnquantisedPolicyNetwork> = unsafe {
        read_into_struct_unchecked("nn-6b5dc1d7fff9.network")
    };

    let quantised = unquantised.quantise();

    let mut file = std::fs::File::create("quantised.network").unwrap();

    unsafe {
        let ptr: *const PolicyNetwork = quantised.as_ref();
        let slice_ptr: *const u8 = std::mem::transmute(ptr);
        let slice = std::slice::from_raw_parts(slice_ptr, std::mem::size_of::<PolicyNetwork>());
        file.write_all(slice).unwrap();
    }
}
