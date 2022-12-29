//! Utilities for working with Java's "Modified UTF-8" character encoding.
//!
//! For more information, refer to [Wikipedia].
//!
//! [Wikipedia]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8

use std::io;
use std::io::Write;
use std::str::from_utf8_unchecked;

use byteorder::{BigEndian, WriteBytesExt};

pub fn write_modified_utf8(mut writer: impl Write, text: &str) -> io::Result<()> {
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            0 => {
                writer.write_u16::<BigEndian>(0xc080)?;
                i += 1;
            }
            b @ 1..=127 => {
                writer.write_u8(b)?;
                i += 1;
            }
            b => {
                let w = utf8_char_width(b);
                debug_assert!(w <= 4);
                debug_assert!(i + w <= bytes.len());

                if w != 4 {
                    writer.write_all(&bytes[i..i + w])?;
                } else {
                    let s = unsafe { from_utf8_unchecked(&bytes[i..i + w]) };
                    let c = s.chars().next().unwrap() as u32 - 0x10000;

                    let s0 = ((c >> 10) as u16) | 0xd800;
                    let s1 = ((c & 0x3ff) as u16) | 0xdc00;

                    writer.write_all(encode_surrogate(s0).as_slice())?;
                    writer.write_all(encode_surrogate(s1).as_slice())?;
                }
                i += w;
            }
        }
    }

    Ok(())
}

const fn utf8_char_width(first_byte: u8) -> usize {
    const UTF8_CHAR_WIDTH: [u8; 256] = [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    UTF8_CHAR_WIDTH[first_byte as usize] as _
}

fn encode_surrogate(surrogate: u16) -> [u8; 3] {
    debug_assert!((0xd800..=0xdfff).contains(&surrogate));

    const TAG_CONT_U8: u8 = 0b1000_0000u8;
    [
        0b11100000 | ((surrogate & 0b11110000_00000000) >> 12) as u8,
        TAG_CONT_U8 | ((surrogate & 0b00001111_11000000) >> 6) as u8,
        TAG_CONT_U8 | (surrogate & 0b00000000_00111111) as u8,
    ]
}

pub fn encoded_len(text: &str) -> usize {
    let mut n = 0;
    let mut i = 0;
    let bytes = text.as_bytes();

    while i < bytes.len() {
        match bytes[i] {
            // Fast path for ASCII here makes a huge difference in benchmarks.
            1..=127 => {
                n += 1;
                i += 1;
            }
            0 => {
                n += 2;
                i += 1;
            }
            b => {
                let w = utf8_char_width(b);

                if w == 4 {
                    n += 6;
                } else {
                    n += w;
                }

                i += w;
            }
        }
    }

    n
}

#[cfg(test)]
#[test]
fn equivalence() {
    fn check(s: &str) {
        let mut ours = vec![];

        let theirs = cesu8::to_java_cesu8(s);
        write_modified_utf8(&mut ours, s).unwrap();

        assert_eq!(theirs, ours);
        assert_eq!(theirs.len(), encoded_len(s));
    }

    check("Mary had a little lamb\0");
    check("🤡💩👻💀☠👽👾🤖🎃😺😸😹😻😼😽🙀😿😾");
    check("ÅÆÇÈØõ÷£¥ý");
}
