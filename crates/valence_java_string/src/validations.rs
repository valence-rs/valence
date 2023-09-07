use std::ops::{Bound, Range, RangeBounds, RangeTo};

use crate::{JavaStr, Utf8Error};

pub(crate) const TAG_CONT: u8 = 0b1000_0000;
pub(crate) const TAG_TWO_B: u8 = 0b1100_0000;
pub(crate) const TAG_THREE_B: u8 = 0b1110_0000;
pub(crate) const TAG_FOUR_B: u8 = 0b1111_0000;
const CONT_MASK: u8 = 0b0011_1111;

#[inline]
const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7f >> width)) as u32
}

#[inline]
const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

#[inline]
const fn utf8_is_cont_byte(byte: u8) -> bool {
    (byte as i8) < -64
}

/// # Safety
///
/// `bytes` must produce a semi-valid UTF-8 string
#[inline]
pub(crate) unsafe fn next_code_point<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I) -> Option<u32> {
    // Decode UTF-8
    let x = *bytes.next()?;
    if x < 128 {
        return Some(x as u32);
    }

    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    // NOTE: Performance is sensitive to the exact formulation here
    let init = utf8_first_byte(x, 2);
    // SAFETY: `bytes` produces an UTF-8-like string,
    // so the iterator must produce a value here.
    let y = unsafe { *bytes.next().unwrap_unchecked() };
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xe0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        // SAFETY: `bytes` produces an UTF-8-like string,
        // so the iterator must produce a value here.
        let z = unsafe { *bytes.next().unwrap_unchecked() };
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xf0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            // SAFETY: `bytes` produces an UTF-8-like string,
            // so the iterator must produce a value here.
            let w = unsafe { *bytes.next().unwrap_unchecked() };
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    Some(ch)
}

/// # Safety
///
/// `bytes` must produce a semi-valid UTF-8 string
#[inline]
pub(crate) unsafe fn next_code_point_reverse<'a, I: DoubleEndedIterator<Item = &'a u8>>(
    bytes: &mut I,
) -> Option<u32> {
    // Decode UTF-8
    let w = match *bytes.next_back()? {
        next_byte if next_byte < 128 => return Some(next_byte as u32),
        back_byte => back_byte,
    };

    // Multibyte case follows
    // Decode from a byte combination out of: [x [y [z w]]]
    let mut ch;
    // SAFETY: `bytes` produces an UTF-8-like string,
    // so the iterator must produce a value here.
    let z = unsafe { *bytes.next_back().unwrap_unchecked() };
    ch = utf8_first_byte(z, 2);
    if utf8_is_cont_byte(z) {
        // SAFETY: `bytes` produces an UTF-8-like string,
        // so the iterator must produce a value here.
        let y = unsafe { *bytes.next_back().unwrap_unchecked() };
        ch = utf8_first_byte(y, 3);
        if utf8_is_cont_byte(y) {
            // SAFETY: `bytes` produces an UTF-8-like string,
            // so the iterator must produce a value here.
            let x = unsafe { *bytes.next_back().unwrap_unchecked() };
            ch = utf8_first_byte(x, 4);
            ch = utf8_acc_cont_byte(ch, y);
        }
        ch = utf8_acc_cont_byte(ch, z);
    }
    ch = utf8_acc_cont_byte(ch, w);

    Some(ch)
}

#[inline(always)]
pub(crate) fn run_utf8_semi_validation(v: &[u8]) -> Result<(), Utf8Error> {
    let mut index = 0;
    let len = v.len();

    let usize_bytes = std::mem::size_of::<usize>();
    let ascii_block_size = 2 * usize_bytes;
    let blocks_end = if len >= ascii_block_size {
        len - ascii_block_size + 1
    } else {
        0
    };
    let align = v.as_ptr().align_offset(usize_bytes);

    while index < len {
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
                // we needed data, but there was none: error!
                if index >= len {
                    err!(None)
                }
                v[index]
            }};
        }

        let first = v[index];
        if first >= 128 {
            let w = utf8_char_width(first);
            // 2-byte encoding is for codepoints  \u{0080} to  \u{07ff}
            //        first  C2 80        last DF BF
            // 3-byte encoding is for codepoints  \u{0800} to  \u{ffff}
            //        first  E0 A0 80     last EF BF BF
            //   INCLUDING surrogates codepoints  \u{d800} to  \u{dfff}
            //               ED A0 80 to       ED BF BF
            // 4-byte encoding is for codepoints \u{1000}0 to \u{10ff}ff
            //        first  F0 90 80 80  last F4 8F BF BF
            //
            // Use the UTF-8 syntax from the RFC
            //
            // https://tools.ietf.org/html/rfc3629
            // UTF8-1      = %x00-7F
            // UTF8-2      = %xC2-DF UTF8-tail
            // UTF8-3      = %xE0 %xA0-BF UTF8-tail / %xE1-EC 2( UTF8-tail ) /
            //               %xED %x80-9F UTF8-tail / %xEE-EF 2( UTF8-tail )
            // UTF8-4      = %xF0 %x90-BF 2( UTF8-tail ) / %xF1-F3 3( UTF8-tail ) /
            //               %xF4 %x80-8F 2( UTF8-tail )
            match w {
                2 => {
                    if next!() as i8 >= -64 {
                        err!(Some(1))
                    }
                }
                3 => {
                    match (first, next!()) {
                        (0xe0, 0xa0..=0xbf) | (0xe1..=0xef, 0x80..=0xbf) => {} /* INCLUDING surrogate codepoints here */
                        _ => err!(Some(1)),
                    }
                    if next!() as i8 >= -64 {
                        err!(Some(2))
                    }
                }
                4 => {
                    match (first, next!()) {
                        (0xf0, 0x90..=0xbf) | (0xf1..=0xf3, 0x80..=0xbf) | (0xf4, 0x80..=0x8f) => {}
                        _ => err!(Some(1)),
                    }
                    if next!() as i8 >= -64 {
                        err!(Some(2))
                    }
                    if next!() as i8 >= -64 {
                        err!(Some(3))
                    }
                }
                _ => err!(Some(1)),
            }
        } else {
            // Ascii case, try to skip forward quickly.
            // When the pointer is aligned, read 2 words of data per iteration
            // until we find a word containing a non-ascii byte.
            if align != usize::MAX && align.wrapping_sub(index) % usize_bytes == 0 {
                let ptr = v.as_ptr();
                while index < blocks_end {
                    // SAFETY: since `align - index` and `ascii_block_size` are
                    // multiples of `usize_bytes`, `block = ptr.add(index)` is
                    // always aligned with a `usize` so it's safe to dereference
                    // both `block` and `block.add(1)`.
                    unsafe {
                        let block = ptr.add(index) as *const usize;
                        // break if there is a nonascii byte
                        let zu = contains_nonascii(*block);
                        let zv = contains_nonascii(*block.add(1));
                        if zu || zv {
                            break;
                        }
                    }
                    index += ascii_block_size;
                }
                // step from the point where the wordwise loop stopped
                while index < len && v[index] < 128 {
                    index += 1;
                }
            } else {
                index += 1;
            }
        }
    }

    Ok(())
}

#[inline(always)]
pub(crate) const fn run_utf8_full_validation_from_semi(v: &[u8]) -> Result<(), Utf8Error> {
    // this function checks for surrogate codepoints, between \u{d800} to \u{dfff},
    // or ED A0 80 to ED BF BF of width 3 unicode chars. The valid range of width 3
    // characters is ED 80 80 to ED BF BF, so we need to check for an ED byte
    // followed by a >=A0 byte.
    let mut index = 0;
    while index + 3 <= v.len() {
        if v[index] == 0xed && v[index + 1] >= 0xa0 {
            return Err(Utf8Error {
                valid_up_to: index,
                error_len: Some(1),
            });
        }
        index += 1;
    }

    Ok(())
}

#[inline]
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

#[inline]
const fn contains_nonascii(x: usize) -> bool {
    const NONASCII_MASK: usize = usize::from_ne_bytes([0x80; std::mem::size_of::<usize>()]);
    (x & NONASCII_MASK) != 0
}

#[cold]
#[track_caller]
pub(crate) fn slice_error_fail(s: &JavaStr, begin: usize, end: usize) -> ! {
    const MAX_DISPLAY_LENGTH: usize = 256;
    let trunc_len = s.floor_char_boundary(MAX_DISPLAY_LENGTH);
    let s_trunc = &s[..trunc_len];
    let ellipsis = if trunc_len < s.len() { "[...]" } else { "" };

    // 1. out of bounds
    if begin > s.len() || end > s.len() {
        let oob_index = if begin > s.len() { begin } else { end };
        panic!("byte index {oob_index} is out of bounds of `{s_trunc}`{ellipsis}");
    }

    // 2. begin <= end
    assert!(
        begin <= end,
        "begin <= end ({} <= {}) when slicing `{}`{}",
        begin,
        end,
        s_trunc,
        ellipsis
    );

    // 3. character boundary
    let index = if !s.is_char_boundary(begin) {
        begin
    } else {
        end
    };
    // find the character
    let char_start = s.floor_char_boundary(index);
    // `char_start` must be less than len and a char boundary
    let ch = s[char_start..].chars().next().unwrap();
    let char_range = char_start..char_start + ch.len_utf8();
    panic!(
        "byte index {} is not a char boundary; it is inside {:?} (bytes {:?}) of `{}`{}",
        index, ch, char_range, s_trunc, ellipsis
    );
}

#[cold]
#[track_caller]
pub(crate) fn str_end_index_len_fail(index: usize, len: usize) -> ! {
    panic!("range end index {index} out of range for JavaStr of length {len}");
}

#[cold]
#[track_caller]
pub(crate) fn str_index_order_fail(index: usize, end: usize) -> ! {
    panic!("JavaStr index starts at {index} but ends at {end}");
}

#[cold]
#[track_caller]
pub(crate) fn str_start_index_overflow_fail() -> ! {
    panic!("attempted to index JavaStr from after maximum usize");
}

#[cold]
#[track_caller]
pub(crate) fn str_end_index_overflow_fail() -> ! {
    panic!("attempted to index JavaStr up to maximum usize")
}

#[inline]
#[track_caller]
pub(crate) fn to_range_checked<R>(range: R, bounds: RangeTo<usize>) -> Range<usize>
where
    R: RangeBounds<usize>,
{
    let len = bounds.end;

    let start = range.start_bound();
    let start = match start {
        Bound::Included(&start) => start,
        Bound::Excluded(start) => start
            .checked_add(1)
            .unwrap_or_else(|| str_start_index_overflow_fail()),
        Bound::Unbounded => 0,
    };

    let end: Bound<&usize> = range.end_bound();
    let end = match end {
        Bound::Included(end) => end
            .checked_add(1)
            .unwrap_or_else(|| str_end_index_overflow_fail()),
        Bound::Excluded(&end) => end,
        Bound::Unbounded => len,
    };

    if start > end {
        str_index_order_fail(start, end);
    }
    if end > len {
        str_end_index_len_fail(end, len);
    }

    Range { start, end }
}
