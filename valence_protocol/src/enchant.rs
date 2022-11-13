// enchant.rs exposes constant values provided by the build script.
// All enchantment variants are located in `EnchantmentKind`. You can use the
// associated const fn functions of `EnchantmentKind` to access details about an
// enchantment type. enchantment specific functions
include!(concat!(env!("OUT_DIR"), "/enchant.rs"));
