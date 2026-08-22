// Command pulse validates a network path with live probes, a compact RTT
// graph and an end-of-run health verdict.

// Layers are ported bottom-up; drop this once the ui layer wires them together.
#![allow(dead_code)]

mod checks;
mod probe;
mod stats;
mod ui;
mod verdict;

fn main() {}
