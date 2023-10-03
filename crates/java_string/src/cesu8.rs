use std::borrow::Cow;

use crate::validations::{utf8_char_width, CONT_MASK, TAG_CONT};
use crate::{JavaStr, JavaString, Utf8Error};

impl JavaStr {
    /// Converts from Java's [modified UTF-8](https://docs.oracle.com/javase/8/docs/api/java/io/DataInput.html#modified-utf-8) format to a `Cow<JavaStr>`.
    #[inline]
    pub fn from_modified_utf8(bytes: &[u8]) -> Result<Cow<JavaStr>, Utf8Error> {
        match JavaStr::from_full_utf8(bytes) {
            Ok(str) => Ok(Cow::Borrowed(str)),
            Err(_) => JavaString::from_modified_utf8_iter(bytes.iter().copied()).map(Cow::Owned),
        }
    }

    /// Converts to Java's [modified UTF-8](https://docs.oracle.com/javase/8/docs/api/java/io/DataInput.html#modified-utf-8) format.
    #[inline]
    #[must_use]
    pub fn to_modified_utf8(&self) -> Cow<[u8]> {
        if is_valid_cesu8(self) {
            Cow::Borrowed(self.as_bytes())
        } else {
            Cow::Owned(self.to_modified_utf8_internal())
        }
    }

    #[inline]
    fn to_modified_utf8_internal(&self) -> Vec<u8> {
        let bytes = self.as_bytes();
        let mut encoded = Vec::with_capacity((bytes.len() + bytes.len()) >> 2);
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == 0 {
                encoded.extend([0xc0, 0x80].into_iter());
                i += 1;
            } else if b < 128 {
                // Pass ASCII through quickly.
                encoded.push(b);
                i += 1;
            } else {
                // Figure out how many bytes we need for this character.
                let w = utf8_char_width(b);
                let char_bytes = unsafe {
                    // SAFETY: input must be valid semi UTF-8, so there must be at least w more
                    // bytes from i
                    bytes.get_unchecked(i..i + w)
                };
                if w != 4 {
                    // Pass through short UTF-8 sequences unmodified.
                    encoded.extend(char_bytes.iter().copied())
                } else {
                    // Encode 4-byte sequences as 6 bytes
                    let s = unsafe {
                        // SAFETY: input is valid semi UTF-8
                        JavaStr::from_semi_utf8_unchecked(bytes)
                    };
                    let c = unsafe {
                        // SAFETY: s contains a single char of width 4
                        s.chars().next().unwrap_unchecked().as_u32() - 0x10000
                    };
                    let s = [((c >> 10) as u16) | 0xd800, ((c & 0x3ff) as u16) | 0xdc00];
                    encoded.extend(enc_surrogate(s[0]).into_iter());
                    encoded.extend(enc_surrogate(s[1]).into_iter());
                }
                i += w;
            }
        }
        encoded
    }
}

impl JavaString {
    /// Converts from Java's [modified UTF-8](https://docs.oracle.com/javase/8/docs/api/java/io/DataInput.html#modified-utf-8) format to a `JavaString`.
    #[inline]
    pub fn from_modified_utf8(bytes: Vec<u8>) -> Result<JavaString, Utf8Error> {
        match JavaString::from_full_utf8(bytes) {
            Ok(str) => Ok(str),
            Err(err) => JavaString::from_modified_utf8_iter(err.bytes.into_iter()),
        }
    }

    /// Converts from Java's [modified UTF-8](https://docs.oracle.com/javase/8/docs/api/java/io/DataInput.html#modified-utf-8) format to a `JavaString`.
    pub fn from_modified_utf8_iter<I>(mut iter: I) -> Result<JavaString, Utf8Error>
    where
        I: Iterator<Item = u8>,
    {
        let mut index = 0;
        let mut decoded = Vec::with_capacity(iter.size_hint().0);
        let mut surrogate_first: Option<[u8; 3]> = None;

        macro_rules! flush_first_surrogate_half {
            () => {
                // append any preceding first half of a surrogate pair
                if let Some(surrogate_first) = surrogate_first.take() {
                    decoded.extend(surrogate_first.into_iter());
                }
            };
        }

        while let Some(first) = iter.next() {
            let old_offset = index;

            macro_rules! err {
                ($error_len:expr) => {
                    return Err(Utf8Error {
                        valid_up_to: old_offset,
                        error_len: $error_len,
                    })
                };
            }

            macro_rules! next {
                () => {{
                    index += 1;
                    match iter.next() {
                        Some(a) => a,
                        None => err!(None),
                    }
                }};
            }

            macro_rules! next_cont {
                ($error_len:expr) => {{
                    let byte = next!();
                    if (byte) & !CONT_MASK == TAG_CONT {
                        byte
                    } else {
                        err!($error_len)
                    }
                }};
            }

            if first == 0 {
                // modified UTF-8 should never contain \0 directly.
                err!(None);
            } else if first < 128 {
                flush_first_surrogate_half!();
                // Pass ASCII through directly.
                decoded.push(first);
            } else if first == 0xc0 {
                flush_first_surrogate_half!();
                // modified UTF-8 encoding of null character
                match next!() {
                    0x80 => decoded.push(0),
                    _ => err!(Some(1)),
                }
            } else {
                let w = utf8_char_width(first);
                let second = next_cont!(Some(1));
                match w {
                    // Two-byte sequences can be used directly.
                    2 => {
                        flush_first_surrogate_half!();
                        decoded.extend([first, second].into_iter());
                    }
                    3 => {
                        let third = next_cont!(Some(2));
                        match (first, second) {
                            // These are valid UTF-8, so pass them through.
                            (0xe0, 0xa0..=0xbf)
                            | (0xe1..=0xec, 0x80..=0xbf)
                            | (0xed, 0x80..=0x9f)
                            | (0xee..=0xef, 0x80..=0xbf) => {
                                flush_first_surrogate_half!();
                                decoded.extend([first, second, third].into_iter())
                            }
                            // First half of a surrogate pair
                            (0xed, 0xa0..=0xaf) => {
                                flush_first_surrogate_half!();
                                surrogate_first = Some([first, second, third]);
                            }
                            // Second half of a surrogate pair
                            (0xed, 0xb0..=0xbf) => {
                                // try to pair the second half with a preceding first half
                                if let Some([_, b, c]) = surrogate_first.take() {
                                    let (fifth, sixth) = (second, third);
                                    let (second, third) = (b, c);
                                    let s = dec_surrogates(second, third, fifth, sixth);
                                    decoded.extend(s.into_iter());
                                } else {
                                    // no first half, append the second half directly
                                    decoded.extend([first, second, third].into_iter());
                                }
                            }
                            _ => err!(Some(1)),
                        }
                    }
                    _ => err!(Some(1)), // modified UTF-8 doesn't allow width 4
                }
            }
        }

        flush_first_surrogate_half!();

        unsafe {
            // SAFETY: we built a semi UTF-8 encoded string
            Ok(JavaString::from_semi_utf8_unchecked(decoded))
        }
    }

    /// Converts to Java's [modified UTF-8](https://docs.oracle.com/javase/8/docs/api/java/io/DataInput.html#modified-utf-8) format.
    #[inline]
    #[must_use]
    pub fn into_modified_utf8(self) -> Vec<u8> {
        if is_valid_cesu8(&self) {
            self.into_bytes()
        } else {
            self.to_modified_utf8_internal()
        }
    }
}

#[inline]
fn dec_surrogate(second: u8, third: u8) -> u32 {
    0xd000 | ((second & CONT_MASK) as u32) << 6 | (third & CONT_MASK) as u32
}

#[inline]
fn dec_surrogates(second: u8, third: u8, fifth: u8, sixth: u8) -> [u8; 4] {
    // Convert to a 32-bit code point.
    let s1 = dec_surrogate(second, third);
    let s2 = dec_surrogate(fifth, sixth);
    let c = 0x10000 + (((s1 - 0xd800) << 10) | (s2 - 0xdc00));
    assert!((0x010000..=0x10ffff).contains(&c));

    // Convert to UTF-8.
    // 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    [
        0b1111_0000u8 | ((c & 0b1_1100_0000_0000_0000_0000) >> 18) as u8,
        TAG_CONT | ((c & 0b0_0011_1111_0000_0000_0000) >> 12) as u8,
        TAG_CONT | ((c & 0b0_0000_0000_1111_1100_0000) >> 6) as u8,
        TAG_CONT | (c & 0b0_0000_0000_0000_0011_1111) as u8,
    ]
}

#[inline]
fn is_valid_cesu8(text: &JavaStr) -> bool {
    text.bytes()
        .all(|b| b != 0 && ((b & !CONT_MASK) == TAG_CONT || utf8_char_width(b) <= 3))
}

#[inline]
fn enc_surrogate(surrogate: u16) -> [u8; 3] {
    // 1110xxxx 10xxxxxx 10xxxxxx
    [
        0b11100000 | ((surrogate & 0b11110000_00000000) >> 12) as u8,
        TAG_CONT | ((surrogate & 0b00001111_11000000) >> 6) as u8,
        TAG_CONT | (surrogate & 0b00000000_00111111) as u8,
    ]
}
