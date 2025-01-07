fn main() {
    #[cfg(feature = "embed")]
    net::run();

    #[cfg(not(feature = "embed"))]
    nonet::run();
}

#[cfg(feature = "embed")]
mod net {
    use monty::{uci, ChessState, MctsParams, PolicyNetwork, ValueNetwork};
    use zstd::stream::decode_all;
    use std::io::Cursor;
    use std::sync::LazyLock;

    // Embed compressed byte arrays
    static COMPRESSED_VALUE: &[u8] = include_bytes!("../value.network.zst");
    static COMPRESSED_POLICY: &[u8] = include_bytes!("../policy.network.zst");    

    static VALUE: LazyLock<Box<ValueNetwork>> = LazyLock::new(|| {
        let decompressed_data = decode_all(Cursor::new(COMPRESSED_VALUE))
            .expect("Failed to decompress value network");
    
        // Allocate the decompressed data on the heap
        Box::new(unsafe { std::ptr::read(decompressed_data.as_ptr() as *const ValueNetwork) })
    });
    
    static POLICY: LazyLock<Box<PolicyNetwork>> = LazyLock::new(|| {
        let decompressed_data = decode_all(Cursor::new(COMPRESSED_POLICY))
            .expect("Failed to decompress policy network");
    
        // Allocate the decompressed data on the heap
        Box::new(unsafe { std::ptr::read(decompressed_data.as_ptr() as *const PolicyNetwork) })
    });

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        if let Some("bench") = arg1.as_deref() {
            uci::bench(
                ChessState::BENCH_DEPTH,
                &POLICY,
                &VALUE,
                &MctsParams::default(),
            );
            return;
        }

        uci::run(&POLICY, &VALUE);
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
