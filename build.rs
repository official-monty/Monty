#[cfg(feature = "embed")]
use sha2::{Digest, Sha256};

#[cfg(feature = "embed")]
use std::fs;

#[cfg(feature = "embed")]
use std::path::Path;

use chrono::Utc;
use std::process::Command;

fn get_name() {
    // Get the current Git commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let git_commit_hash = String::from_utf8(output.stdout)
        .expect("Git output was not valid UTF-8")
        .trim()
        .to_string();

    // Get the current date in YYYYMMDD format
    let current_date = Utc::now().format("%Y%m%d").to_string();

    // Combine into the desired format
    let formatted_name = format!("Monty-dev-{}-{}", current_date, &git_commit_hash[..8]);

    // Pass the formatted name as an environment variable
    println!("cargo:rustc-env=FORMATTED_NAME={}", formatted_name);
}

#[cfg(feature = "embed")]
fn main() {
    // Get the build version name
    get_name();

    // Extract the file names from the respective source files
    let value_file_name = extract_network_name("src/networks/value.rs", "ValueFileDefaultName");
    let policy_file_name = extract_network_name("src/networks/policy.rs", "PolicyFileDefaultName");

    // Define fixed paths where the networks will be stored
    let value_path = "value.network";
    let policy_path = "policy.network";

    // Validate and download the network files if needed
    validate_and_download_network(&value_file_name, &value_path);
    validate_and_download_network(&policy_file_name, &policy_path);

    // Set up cargo instructions to track changes
    println!("cargo:rerun-if-changed=src/networks/value.rs");
    println!("cargo:rerun-if-changed=src/networks/policy.rs");
    println!("cargo:rerun-if-changed={}", value_path);
    println!("cargo:rerun-if-changed={}", policy_path);
}

#[cfg(not(feature = "embed"))]
fn main() {
    // Get the build version name
    get_name();
}

#[cfg(feature = "embed")]
fn extract_network_name(file_path: &str, const_name: &str) -> String {
    let content = fs::read_to_string(file_path).expect("Unable to read networks file");

    for line in content.lines() {
        if line.contains(const_name) {
            // Split the line on the '=' character to separate the variable name and the value
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                // Further split on '"' to extract the string value
                let network_name = parts[1].split('"').nth(1);
                if let Some(name) = network_name {
                    return name.into();
                }
            }
        }
    }
    panic!(
        "Network name not found or could not be parsed in {}",
        file_path
    );
}

#[cfg(feature = "embed")]
fn validate_and_download_network(expected_name: &str, dest_path: &str) {
    let path = Path::new(dest_path);
    // Append `.zst` to the dest_path
    let compressed_path_string = format!("{}.zst", dest_path);
    let compressed_path = Path::new(&compressed_path_string);

    // Extract the expected SHA-256 prefix from the expected file name
    let expected_prefix = extract_sha_prefix(expected_name);

    // If the file exists, calculate its SHA-256 and check the first 12 characters
    if path.exists() && compressed_path.exists() {
        if let Ok(existing_sha) = calculate_sha256(path) {
            println!("Expected SHA-256 prefix: {}", expected_prefix);
            println!("Actual SHA-256: {}", &existing_sha[..12]);

            if existing_sha.starts_with(&expected_prefix) {
                println!(
                    "File at {} is valid with matching SHA-256 prefix.",
                    dest_path
                );
                return; // No need to download
            } else {
                println!(
                    "File at {} has a mismatching SHA-256 prefix, redownloading...",
                    dest_path
                );
            }
        } else {
            println!(
                "Failed to calculate SHA-256 for {}, redownloading...",
                dest_path
            );
        }
    }

    // Download the correct network file
    download_network(expected_name, dest_path);

    // Zstd compress the downloaded file
    let compressed_path = format!("{}.zst", dest_path);
    compress_with_zstd(dest_path, &compressed_path);
}

#[cfg(feature = "embed")]
fn compress_with_zstd(input_path: &str, output_path: &str) {
    use std::fs::{File};
    use std::io::{BufReader, BufWriter, copy};
    use zstd::Encoder;

    let input_file = File::open(input_path).expect("Failed to open input file");
    let output_file = File::create(output_path).expect("Failed to create output file");

    let mut reader = BufReader::new(input_file);
    let writer = BufWriter::new(output_file);

    // Initialize the Zstd encoder with compression level 22
    let mut encoder = Encoder::new(writer, 22).expect("Failed to create Zstd encoder");
    // Use 4 threads
    encoder.multithread(4).expect("Failed to set multithreaded compression");

    // Copy the data from the reader and write to the encoder
    copy(&mut reader, &mut encoder).expect("Failed to compress data");

    // Finalize the compression process
    encoder.finish().expect("Failed to finish compression");
}

#[cfg(feature = "embed")]
fn extract_sha_prefix(file_name: &str) -> String {
    // Assume the format is "nn-<sha_prefix>.network"
    let parts: Vec<&str> = file_name.split('-').collect();
    if parts.len() == 2 {
        return parts[1][..12].to_string(); // Extract the SHA-256 prefix
    }
    panic!("Invalid file name format: {}", file_name);
}

#[cfg(feature = "embed")]
fn calculate_sha256(path: &Path) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(feature = "embed")]
fn download_network(network_name: &str, dest_path: &str) {
    let urls = [format!(
        "https://tests.montychess.org/api/nn/{}",
        network_name
    )];

    for url in &urls {
        let output = Command::new("curl")
            .arg("-sL")
            .arg(url)
            .output()
            .expect("Failed to execute curl");

        if output.status.success() {
            fs::write(dest_path, output.stdout).expect("Failed to write network file");
            println!("Downloaded {}", dest_path);
            return;
        }
    }
    panic!("Failed to download network file from any source.");
}
