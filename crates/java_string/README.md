# java_string

An implementation of Java strings, tolerant of invalid UTF-16 encoding.
This allows for round-trip serialization of all Java strings, including those which contain invalid UTF-16, while still
being able to perform useful operations on those strings. 

These Java strings use the UTF-8 encoding, with the modification that surrogate code points (code points between U+D800 
and U+DFFF inclusive) are allowed. This allows for zero-cost conversion from Rust strings to Java strings. This modified
encoding is known as "semi-UTF-8" throughout the codebase. Similarly, this crate introduces a `JavaCodePoint` type which
is analogous to `char`, except that surrogate code points are allowed.

This crate is mostly undocumented, because most methods are entirely analogous to those of the same name in Rust's
strings. Please refer to the `std` documentation.

# Features

- `serde` Adds support for [`serde`](https://docs.rs/serde/latest/serde/)