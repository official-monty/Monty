#[cfg(not(feature = "nonet"))]
use std::env;

#[cfg(not(feature = "nonet"))]
const DEFAULT_PATH: &str = "resources/net.network";

#[cfg(not(feature = "nonet"))]
fn main() {
    println!("cargo:rerun-if-env-changed=EVALFILE");
    println!("cargo:rerun-if-changed=resources/chess.network");
    println!("cargo:rerun-if-changed=chess.network");
    let net_path = env::var("EVALFILE").unwrap_or(DEFAULT_PATH.into());
    if net_path != DEFAULT_PATH {
        std::fs::copy(net_path, DEFAULT_PATH).unwrap();
    }
}

#[cfg(feature = "nonet")]
fn main() {}
