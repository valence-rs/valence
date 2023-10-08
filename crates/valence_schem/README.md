# valence_schem

Support for the [Sponge schematic file format](https://github.com/SpongePowered/Schematic-Specification).

This crate implements [Sponge schematics]

Loading schematics (version 1 through 3) from [`Compounds`](Compound) is
supported. Saving schematics to [`Compounds`](Compound) (version 3 only) is
supported.

# Examples

An example that shows how to load and save [schematics] from and to the
filesystem

```rust
# use valence_schem::Schematic;
use flate2::Compression;
fn schem_from_file(path: &str) -> Schematic {
    Schematic::load(path).unwrap()
}
fn schem_to_file(schematic: &Schematic, path: &str) {
    schematic.save(path);
}
```

There are also methods to serialize and deserialize [schematics] from and to
[`Compounds`](Compound):
```rust
# use valence_schem::Schematic;
use valence_nbt::Compound;
fn schem_from_compound(compound: &Compound) {
    let schematic = Schematic::deserialize(compound).unwrap();
    let comp = schematic.serialize();
}
```

### See also

Examples in the `examples/` directory

[Sponge schematics]: <https://github.com/SpongePowered/Schematic-Specification>
[schematics]: Schematic