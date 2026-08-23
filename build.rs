fn main() {
    // The Makefile injects the version; without it the binary reports "dev",
    // matching a plain `cargo build`.
    println!("cargo:rerun-if-env-changed=PULSE_VERSION");
}
