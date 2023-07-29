#![cfg_attr(
    all(not(debug_assertions), feature = "gui"),
    windows_subsystem = "windows"
)]

#[cfg(any(
    all(not(feature = "gui"), not(feature = "cli")),
    all(feature = "gui", feature = "cli")
))]
fn main() {
    panic!("Invalid features; select either \"cli\" or \"gui\"");
}

#[cfg(all(not(feature = "cli"), feature = "gui"))]
include!("./main_gui.rs");

#[cfg(all(not(feature = "gui"), feature = "cli"))]
include!("./main_cli.rs");
