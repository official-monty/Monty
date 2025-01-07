fn main() {
    #[cfg(feature = "embed")]
    net::run();

    #[cfg(not(feature = "embed"))]
    nonet::run();
}

#[cfg(feature = "embed")]
mod net {
    use monty::{uci, ChessState, MctsParams, PolicyNetwork, ValueNetwork};
    use std::io::Cursor;
    use std::mem::MaybeUninit;
    use std::sync::LazyLock;
    use zstd::stream::decode_all;

    // Embed compressed byte arrays
    static COMPRESSED_VALUE: &[u8] = include_bytes!("../value.network.zst");
    static COMPRESSED_POLICY: &[u8] = include_bytes!("../policy.network.zst");    

    /// Helper function to safely decompress and initialize a Boxed structure.
    fn decompress_into_boxed<T>(data: &[u8]) -> Box<T> {
        // Ensure the decompressed data size matches the target structure size
        assert_eq!(
            data.len(),
            std::mem::size_of::<T>(),
            "Decompressed data size does not match the target structure size."
        );

        // Create an uninitialized Box
        let mut boxed = Box::new(MaybeUninit::<T>::uninit());

        unsafe {
            // Copy the decompressed data into the Box's memory
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                boxed.as_mut_ptr() as *mut u8,
                data.len(),
            );

            // Assume the Box is now initialized
            boxed.assume_init()
        }
    }

    // Lazy initialization for VALUE using LazyLock to ensure heap allocation
    static VALUE: LazyLock<Box<ValueNetwork>> = LazyLock::new(|| {
        // Decompress the value network
        let decompressed_data = decode_all(Cursor::new(COMPRESSED_VALUE))
            .expect("Failed to decompress value network");

        // Initialize the Box<ValueNetwork> with the decompressed data
        decompress_into_boxed::<ValueNetwork>(&decompressed_data)
    });

    // Lazy initialization for POLICY using LazyLock to ensure heap allocation
    static POLICY: LazyLock<Box<PolicyNetwork>> = LazyLock::new(|| {
        // Decompress the policy network
        let decompressed_data = decode_all(Cursor::new(COMPRESSED_POLICY))
            .expect("Failed to decompress policy network");

        // Initialize the Box<PolicyNetwork> with the decompressed data
        decompress_into_boxed::<PolicyNetwork>(&decompressed_data)
    });

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        if let Some("bench") = arg1.as_deref() {
            uci::bench(
                ChessState::BENCH_DEPTH,
                &*POLICY, // Dereference the Box to get &PolicyNetwork
                &*VALUE,  // Dereference the Box to get &ValueNetwork
                &MctsParams::default(),
            );
            return;
        }

        uci::run(&*POLICY, &*VALUE);
    }
}

#[cfg(not(feature = "embed"))]
mod nonet {
    use monty::{read_into_struct_unchecked, uci, ChessState, MappedWeights, MctsParams};

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        let policy_mapped: MappedWeights<monty::PolicyNetwork> =
            unsafe { read_into_struct_unchecked(monty::PolicyFileDefaultName) };

        let value_mapped: MappedWeights<monty::ValueNetwork> =
            unsafe { read_into_struct_unchecked(monty::ValueFileDefaultName) };

        let policy = policy_mapped.data;
        let value = value_mapped.data;

        if let Some("bench") = arg1.as_deref() {
            uci::bench(
                ChessState::BENCH_DEPTH,
                policy,
                value,
                &MctsParams::default(),
            );
            return;
        }

        uci::run(policy, value);
    }
}
