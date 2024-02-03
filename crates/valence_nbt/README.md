# valence_nbt

A library for encoding and decoding Minecraft's [Named Binary Tag] (NBT)
format.

[Named Binary Tag]: https://minecraft.wiki/w/NBT_format

# Features
- `binary`: Adds support for serializing and deserializing in Java edition's binary format.
- `snbt`: Adds support for serializing and deserializing in "stringified" format.
- `preserve_order`: Causes the order of fields in [`Compound`]s to be
preserved during insertion and deletion at a slight cost to performance.
The iterators on `Compound` can then implement [`DoubleEndedIterator`].
- `serde` Adds support for [`serde`](https://docs.rs/serde/latest/serde/)
