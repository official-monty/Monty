use std::fs;
use std::path::Path;
use std::process::Command;

#[cfg(feature = "embed")]
fn main() {
    // Extract the file names from the respective source files
    let value_file_name = extract_network_name("src/networks/value.rs", "ValueFileDefaultName");
    let policy_file_name = extract_network_name("src/networks/policy.rs", "PolicyFileDefaultName");

    // Define paths where the networks will be stored
    let value_path = format!("resources/{}", value_file_name);
    let policy_path = format!("resources/{}", policy_file_name);

    // Download the network files if they do not exist
    download_network_if_needed(&value_file_name, &value_path);
    download_network_if_needed(&policy_file_name, &policy_path);

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
    panic!("Network name not found or could not be parsed in {}", file_path);
}

#[cfg(feature = "embed")]
fn download_network_if_needed(network_name: &str, dest_path: &str) {
    let urls = [
        format!("https://montychess.org/api/nn/{}", network_name),
    ];

    let path = Path::new(dest_path);
    if !path.exists() {
        for url in &urls {
            let output = Command::new("curl")
                .arg("-sL")
                .arg(url)
                .output()
                .expect("Failed to execute curl");

            if output.status.success() {
                fs::write(&path, output.stdout).expect("Failed to write network file");
                println!("Downloaded {}", path.display());
                break;
            }
        }
    }
}
