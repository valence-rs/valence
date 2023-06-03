# valence_nbt

A library for encoding and decoding Minecraft's [Named Binary Tag] (NBT)
format.

[Named Binary Tag]: https://minecraft.fandom.com/wiki/NBT_format

# Examples

Encode NBT data to its binary form. We are using the [`compound!`] macro to
conveniently construct [`Compound`] values.

```rust
use valence_nbt::{compound, Compound, List};

let c = compound! {
    "byte" => 5_i8,
    "string" => "hello",
    "list_of_float" => List::Float(vec![
        3.1415,
        2.7182,
        1.4142
    ]),
};

let mut buf = vec![];

c.to_binary(&mut buf, "").unwrap();
```

Decode NBT data from its binary form.

```rust
use valence_nbt::{compound, Compound};

let some_bytes = [10, 0, 0, 3, 0, 3, 105, 110, 116, 0, 0, 222, 173, 0];

let expected_value = compound! {
    "int" => 0xdead
};

let (nbt, root_name) = Compound::from_binary(&mut some_bytes.as_slice()).unwrap();

assert_eq!(nbt, expected_value);
assert_eq!(root_name, "");
```

# Features

- `preserve_order`: Causes the order of fields in [`Compound`]s to be
preserved during insertion and deletion at a slight cost to performance.
The iterators on `Compound` can then implement [`DoubleEndedIterator`].
