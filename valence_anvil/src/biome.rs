//! This module contains data for the default Minecraft biomes.
//!
//! All biome variants are located in [`BiomeKind`]. You can use the
//! associated const functions of [`BiomeKind`] to access details about a
//! biome type.

include!(concat!(env!("OUT_DIR"), "/biome.rs"));
