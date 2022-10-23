// biome.rs exposes constant values provided by the build script.
// All biome variants are located in `BiomeKind`. You can use the
// associated const fn functions of `BiomeKind` to access details about a biome
// type.
include!(concat!(env!("OUT_DIR"), "/biome.rs"));
