#[cfg(feature = "embed")]
use sha2::{Digest, Sha256};

#[cfg(feature = "embed")]
use std::fs;

#[cfg(feature = "embed")]
use std::path::Path;

#[cfg(feature = "embed")]
use std::process::Command;

#[cfg(feature = "embed")]
fn main() {
    // Extract the file names from the respective source files
    let value_file_name = extract_network_name("src/networks/value.rs", "ValueFileDefaultName");
    let policy_file_name = extract_network_name("src/networks/policy.rs", "PolicyFileDefaultName");

    // Define fixed paths where the networks will be stored
    let value_path = "resources/value.network";
    let policy_path = "resources/policy.network";

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
fn main() {}

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

    // Extract the expected SHA-256 prefix from the expected file name
    let expected_prefix = extract_sha_prefix(expected_name);

    // If the file exists, calculate its SHA-256 and check the first 12 characters
    if path.exists() {
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
    let urls = [format!("https://montychess.org/api/nn/{}", network_name)];

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
