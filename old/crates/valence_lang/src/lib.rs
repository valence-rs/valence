#![doc = include_str!("../README.md")]

/// Contains Rust constants for all of Minecraft's standard translation keys.
///
/// Use these with `Text::translate`.
pub mod keys {
    include!(concat!(env!("OUT_DIR"), "/translation_keys.rs"));
}
