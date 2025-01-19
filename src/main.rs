fn main() {
    #[cfg(feature = "embed")]
    net::run();

    #[cfg(not(feature = "embed"))]
    nonet::run();
}

#[cfg(feature = "embed")]
mod net {
    use memmap2::Mmap;
    use monty::{
        chess::ChessState,
        mcts::MctsParams,
        networks::{PolicyNetwork, ValueNetwork},
        uci,
    };
    use once_cell::sync::Lazy;
    use sha2::{Digest, Sha256};
    use std::fs::{self, File};
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;
    use zstd::stream::decode_all;

    // Embed compressed byte arrays
    static COMPRESSED_VALUE: &[u8] = include_bytes!("../value.network.zst");
    static COMPRESSED_POLICY: &[u8] = include_bytes!("../policy.network.zst");

    /// Compute the first 12 hexadecimal characters of the SHA-256 hash of the data.
    fn compute_short_sha(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        // Convert the hash to a hexadecimal string and take the first 12 characters
        format!("{:x}", result)[..12].to_string()
    }

    /// Get the full path in the OS's temporary directory for the given data.
    /// The filename format is "nn-<hash_prefix>.network"
    fn get_network_path(data: &[u8]) -> PathBuf {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("Monty");
        fs::create_dir_all(&temp_dir)
            .expect("Failed to create 'Monty' directory in the temp folder");
        let hash_prefix = compute_short_sha(data);
        temp_dir.join(format!("nn-{}.network", hash_prefix))
    }

    /// Extract the first 12 characters of the SHA-256 prefix from the filename.
    /// Assumes the filename format is "nn-<hash_prefix>.network"
    fn extract_sha_prefix(file_name: &str) -> String {
        // Ensure the filename starts with "nn-" and ends with ".network"
        if file_name.starts_with("nn-") && file_name.ends_with(".network") {
            // Extract the hash prefix
            let start = 3; // Length of "nn-"
            let end = file_name.len() - ".network".len();
            let hash_prefix = &file_name[start..end];
            if hash_prefix.len() == 12 {
                return hash_prefix.to_string();
            }
        }
        panic!("Invalid file name format: {}", file_name);
    }

    /// Cleanup old decompressed network files, ensuring that:
    /// - Files matching `current_hash_prefixes` are never deleted.
    /// - Only up to 6 non-matching files are retained, deleting the oldest ones beyond this limit.
    fn cleanup_old_files(current_hash_prefixes: &[&str]) -> io::Result<()> {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("Monty");
        fs::create_dir_all(&temp_dir)
            .expect("Failed to create 'Monty' directory in the temp folder");

        // Vectors to hold (path, modified_time) tuples
        let mut matching_files: Vec<(fs::DirEntry, SystemTime)> = Vec::new();
        let mut non_matching_files: Vec<(fs::DirEntry, SystemTime)> = Vec::new();

        for entry in fs::read_dir(&temp_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                    // Check if the file matches the naming pattern
                    if fname.starts_with("nn-") && fname.ends_with(".network") {
                        // Extract the hash prefix from the filename
                        let extracted_hash = extract_sha_prefix(fname);
                        // Get the file's metadata to retrieve the modification time
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified_time) = metadata.modified() {
                                if current_hash_prefixes.contains(&extracted_hash.as_str()) {
                                    // This file matches a current hash prefix; preserve it
                                    matching_files.push((entry, modified_time));
                                } else {
                                    // This file does not match; consider it for cleanup
                                    non_matching_files.push((entry, modified_time));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort non-matching files by modification time (oldest first)
        non_matching_files.sort_by_key(|(_, mtime)| *mtime);

        // Calculate how many non-matching files to delete
        let excess_non_matching = non_matching_files.len().saturating_sub(6);

        if excess_non_matching > 0 {
            for (entry, _) in non_matching_files.into_iter().take(excess_non_matching) {
                let path = entry.path();
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("Failed to delete {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Decompress the data and write it to the specified file path.
    /// If the file already exists and its hash prefix matches, do nothing.
    /// Otherwise, decompress and write the file.
    fn decompress_and_write(
        _network_type: &str,
        compressed_data: &[u8],
        file_path: &Path,
    ) -> std::io::Result<()> {
        // Compute expected hash prefix
        let expected_hash_prefix = compute_short_sha(compressed_data);

        // Check if a file with the expected hash prefix already exists
        if file_path.exists() {
            // Extract the existing file's hash prefix
            let existing_file_name = file_path.file_name().unwrap().to_str().unwrap();
            let existing_hash_prefix = extract_sha_prefix(existing_file_name);

            if existing_hash_prefix == expected_hash_prefix {
                // Hash prefix matches; no need to overwrite
                return Ok(());
            } else {
                // Hash prefix mismatch; remove the old file
                fs::remove_file(file_path)?;
            }
        }

        // Decompress the data
        let decompressed_data = decode_all(Cursor::new(compressed_data)).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Decompression failed: {}", e),
            )
        })?;

        // Write the decompressed data to a temporary file first
        let temp_file_path = file_path.with_extension("tmp");
        {
            let mut temp_file = File::create(&temp_file_path)?;
            temp_file.write_all(&decompressed_data)?;
        }

        // Atomically rename the temporary file to the target path
        fs::rename(&temp_file_path, file_path)?;

        Ok(())
    }

    /// Unsafe helper function to interpret the memory-mapped data as the target structure.
    /// Ensure that the data layout matches exactly.
    unsafe fn read_into_struct_unchecked<T>(mmap: &Mmap) -> &T {
        assert_eq!(
            mmap.len(),
            std::mem::size_of::<T>(),
            "Mapped file size does not match the target structure size."
        );
        &*(mmap.as_ptr() as *const T)
    }

    // Initialize and memory-map both policy and value networks together
    static NETWORKS: Lazy<(Mmap, Mmap)> = Lazy::new(|| {
        // Compute hash prefixes based on compressed data
        let policy_hash_prefix = compute_short_sha(COMPRESSED_POLICY);
        let value_hash_prefix = compute_short_sha(COMPRESSED_VALUE);

        // Current hash prefixes
        let current_hash_prefixes = [policy_hash_prefix.as_str(), value_hash_prefix.as_str()];

        // Cleanup old network files not matching current hash prefixes
        cleanup_old_files(&current_hash_prefixes).expect("Failed to cleanup old network files");

        // Get file paths in the temporary directory
        let policy_path = get_network_path(COMPRESSED_POLICY);
        let value_path = get_network_path(COMPRESSED_VALUE);

        // Decompress and write network files
        decompress_and_write("policy", COMPRESSED_POLICY, &policy_path)
            .expect("Failed to decompress/write policy network");

        decompress_and_write("value", COMPRESSED_VALUE, &value_path)
            .expect("Failed to decompress/write value network");

        // Memory-map the policy network file
        let policy_file =
            File::open(&policy_path).expect("Failed to open policy network file for mmap");
        let policy_mmap =
            unsafe { Mmap::map(&policy_file).expect("Failed to memory-map policy network file") };

        // Memory-map the value network file
        let value_file =
            File::open(&value_path).expect("Failed to open value network file for mmap");
        let value_mmap =
            unsafe { Mmap::map(&value_file).expect("Failed to memory-map value network file") };

        (policy_mmap, value_mmap)
    });

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        // Interpret the memory-mapped data as network structures
        let policy: &PolicyNetwork = unsafe { read_into_struct_unchecked(&NETWORKS.0) };
        let value: &ValueNetwork = unsafe { read_into_struct_unchecked(&NETWORKS.1) };

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

#[cfg(not(feature = "embed"))]
mod nonet {
    use monty::{
        chess::ChessState, mcts::MctsParams, networks, read_into_struct_unchecked, uci,
        MappedWeights,
    };

    pub fn run() {
        let mut args = std::env::args();
        let arg1 = args.nth(1);

        let policy_mapped: MappedWeights<networks::PolicyNetwork> =
            unsafe { read_into_struct_unchecked(networks::PolicyFileDefaultName) };

        let value_mapped: MappedWeights<networks::ValueNetwork> =
            unsafe { read_into_struct_unchecked(networks::ValueFileDefaultName) };

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
