use std::char::ParseCharError;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::{once, FusedIterator, Once};
use std::ops::Range;
use std::str::FromStr;

use crate::validations::{TAG_CONT, TAG_FOUR_B, TAG_THREE_B, TAG_TWO_B};

// JavaCodePoint is guaranteed to have the same repr as a u32, with valid values
// of between 0 and 0x10FFFF, the same as a unicode code point. Surrogate code
// points are valid values of this type.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct JavaCodePoint {
    #[cfg(target_endian = "little")]
    lower: u16,
    upper: SeventeenValues,
    #[cfg(target_endian = "big")]
    lower: u16,
}

#[repr(u16)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(unused)]
enum SeventeenValues {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    V16,
}

impl JavaCodePoint {
    pub const MAX: JavaCodePoint = JavaCodePoint::from_char(char::MAX);
    pub const REPLACEMENT_CHARACTER: JavaCodePoint =
        JavaCodePoint::from_char(char::REPLACEMENT_CHARACTER);

    /// See [`char::from_u32`]
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// let c = JavaCodePoint::from_u32(0x2764);
    /// assert_eq!(Some(JavaCodePoint::from_char('❤')), c);
    ///
    /// assert_eq!(None, JavaCodePoint::from_u32(0x110000));
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_u32(i: u32) -> Option<JavaCodePoint> {
        if i <= 0x10ffff {
            unsafe { Some(Self::from_u32_unchecked(i)) }
        } else {
            None
        }
    }

    /// # Safety
    /// The argument must be within the valid Unicode code point range of 0 to
    /// 0x10FFFF inclusive. Surrogate code points are allowed.
    #[inline]
    #[must_use]
    pub const unsafe fn from_u32_unchecked(i: u32) -> JavaCodePoint {
        // SAFETY: the caller checks that the argument can be represented by this type
        std::mem::transmute(i)
    }

    /// Converts a `char` to a code point.
    #[inline]
    #[must_use]
    pub const fn from_char(char: char) -> JavaCodePoint {
        unsafe {
            // SAFETY: all chars are valid code points
            JavaCodePoint::from_u32_unchecked(char as u32)
        }
    }

    /// Converts this code point to a `u32`.
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(65, JavaCodePoint::from_char('A').as_u32());
    /// assert_eq!(0xd800, JavaCodePoint::from_u32(0xd800).unwrap().as_u32());
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        unsafe {
            // SAFETY: JavaCodePoint has the same repr as a u32
            let result = std::mem::transmute::<Self, u32>(self);

            if result > 0x10ffff {
                // SAFETY: JavaCodePoint can never have a value > 0x10FFFF.
                // This statement may allow the optimizer to remove branches in the calling code
                // associated with out of bounds chars.
                std::hint::unreachable_unchecked();
            }

            result
        }
    }

    /// Converts this code point to a `char`.
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(Some('a'), JavaCodePoint::from_char('a').as_char());
    /// assert_eq!(None, JavaCodePoint::from_u32(0xd800).unwrap().as_char());
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_char(self) -> Option<char> {
        char::from_u32(self.as_u32())
    }

    /// # Safety
    /// The caller must ensure that this code point is not a surrogate code
    /// point.
    #[inline]
    #[must_use]
    pub unsafe fn as_char_unchecked(self) -> char {
        char::from_u32_unchecked(self.as_u32())
    }

    /// See [`char::encode_utf16`]
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(
    ///     2,
    ///     JavaCodePoint::from_char('𝕊')
    ///         .encode_utf16(&mut [0; 2])
    ///         .len()
    /// );
    /// assert_eq!(
    ///     1,
    ///     JavaCodePoint::from_u32(0xd800)
    ///         .unwrap()
    ///         .encode_utf16(&mut [0; 2])
    ///         .len()
    /// );
    /// ```
    /// ```should_panic
    /// # use java_string::JavaCodePoint;
    /// // Should panic
    /// JavaCodePoint::from_char('𝕊').encode_utf16(&mut [0; 1]);
    /// ```
    #[inline]
    pub fn encode_utf16(self, dst: &mut [u16]) -> &mut [u16] {
        if let Some(char) = self.as_char() {
            char.encode_utf16(dst)
        } else {
            dst[0] = self.as_u32() as u16;
            &mut dst[..1]
        }
    }

    /// Encodes this `JavaCodePoint` into semi UTF-8, that is, UTF-8 with
    /// surrogate code points. See also [`char::encode_utf8`].
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(
    ///     2,
    ///     JavaCodePoint::from_char('ß')
    ///         .encode_semi_utf8(&mut [0; 4])
    ///         .len()
    /// );
    /// assert_eq!(
    ///     3,
    ///     JavaCodePoint::from_u32(0xd800)
    ///         .unwrap()
    ///         .encode_semi_utf8(&mut [0; 4])
    ///         .len()
    /// );
    /// ```
    /// ```should_panic
    /// # use java_string::JavaCodePoint;
    /// // Should panic
    /// JavaCodePoint::from_char('ß').encode_semi_utf8(&mut [0; 1]);
    /// ```
    #[inline]
    pub fn encode_semi_utf8(self, dst: &mut [u8]) -> &mut [u8] {
        let len = self.len_utf8();
        let code = self.as_u32();
        match (len, &mut dst[..]) {
            (1, [a, ..]) => {
                *a = code as u8;
            }
            (2, [a, b, ..]) => {
                *a = ((code >> 6) & 0x1f) as u8 | TAG_TWO_B;
                *b = (code & 0x3f) as u8 | TAG_CONT;
            }
            (3, [a, b, c, ..]) => {
                *a = ((code >> 12) & 0x0f) as u8 | TAG_THREE_B;
                *b = ((code >> 6) & 0x3f) as u8 | TAG_CONT;
                *c = (code & 0x3f) as u8 | TAG_CONT;
            }
            (4, [a, b, c, d, ..]) => {
                *a = ((code >> 18) & 0x07) as u8 | TAG_FOUR_B;
                *b = ((code >> 12) & 0x3f) as u8 | TAG_CONT;
                *c = ((code >> 6) & 0x3f) as u8 | TAG_CONT;
                *d = (code & 0x3f) as u8 | TAG_CONT;
            }
            _ => panic!(
                "encode_utf8: need {} bytes to encode U+{:X}, but the buffer has {}",
                len,
                code,
                dst.len()
            ),
        }
        &mut dst[..len]
    }

    /// See [`char::eq_ignore_ascii_case`].
    #[inline]
    pub fn eq_ignore_ascii_case(&self, other: &JavaCodePoint) -> bool {
        match (self.as_char(), other.as_char()) {
            (Some(char1), Some(char2)) => char1.eq_ignore_ascii_case(&char2),
            (None, None) => self == other,
            _ => false,
        }
    }

    /// See [`char::escape_debug`].
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(
    ///     "a",
    ///     JavaCodePoint::from_char('a').escape_debug().to_string()
    /// );
    /// assert_eq!(
    ///     "\\n",
    ///     JavaCodePoint::from_char('\n').escape_debug().to_string()
    /// );
    /// assert_eq!(
    ///     "\\u{d800}",
    ///     JavaCodePoint::from_u32(0xd800)
    ///         .unwrap()
    ///         .escape_debug()
    ///         .to_string()
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn escape_debug(self) -> CharEscapeIter {
        self.escape_debug_ext(EscapeDebugExtArgs::ESCAPE_ALL)
    }

    #[inline]
    #[must_use]
    pub(crate) fn escape_debug_ext(self, args: EscapeDebugExtArgs) -> CharEscapeIter {
        const NULL: u32 = '\0' as u32;
        const TAB: u32 = '\t' as u32;
        const CARRIAGE_RETURN: u32 = '\r' as u32;
        const LINE_FEED: u32 = '\n' as u32;
        const SINGLE_QUOTE: u32 = '\'' as u32;
        const DOUBLE_QUOTE: u32 = '"' as u32;
        const BACKSLASH: u32 = '\\' as u32;

        unsafe {
            // SAFETY: all characters specified are in ascii range
            match self.as_u32() {
                NULL => CharEscapeIter::new([b'\\', b'0']),
                TAB => CharEscapeIter::new([b'\\', b't']),
                CARRIAGE_RETURN => CharEscapeIter::new([b'\\', b'r']),
                LINE_FEED => CharEscapeIter::new([b'\\', b'n']),
                SINGLE_QUOTE if args.escape_single_quote => CharEscapeIter::new([b'\\', b'\'']),
                DOUBLE_QUOTE if args.escape_double_quote => CharEscapeIter::new([b'\\', b'"']),
                BACKSLASH => CharEscapeIter::new([b'\\', b'\\']),
                _ if self.is_printable() => {
                    // SAFETY: surrogate code points are not printable
                    CharEscapeIter::printable(self.as_char_unchecked())
                }
                _ => self.escape_unicode(),
            }
        }
    }

    #[inline]
    fn is_printable(self) -> bool {
        let Some(char) = self.as_char() else {
            return false;
        };
        if matches!(char, '\\' | '\'' | '"') {
            return true;
        }
        char.escape_debug().next() != Some('\\')
    }

    /// See [`char::escape_default`].
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(
    ///     "a",
    ///     JavaCodePoint::from_char('a').escape_default().to_string()
    /// );
    /// assert_eq!(
    ///     "\\n",
    ///     JavaCodePoint::from_char('\n').escape_default().to_string()
    /// );
    /// assert_eq!(
    ///     "\\u{d800}",
    ///     JavaCodePoint::from_u32(0xd800)
    ///         .unwrap()
    ///         .escape_default()
    ///         .to_string()
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn escape_default(self) -> CharEscapeIter {
        const TAB: u32 = '\t' as u32;
        const CARRIAGE_RETURN: u32 = '\r' as u32;
        const LINE_FEED: u32 = '\n' as u32;
        const SINGLE_QUOTE: u32 = '\'' as u32;
        const DOUBLE_QUOTE: u32 = '"' as u32;
        const BACKSLASH: u32 = '\\' as u32;

        unsafe {
            // SAFETY: all characters specified are in ascii range
            match self.as_u32() {
                TAB => CharEscapeIter::new([b'\\', b't']),
                CARRIAGE_RETURN => CharEscapeIter::new([b'\\', b'r']),
                LINE_FEED => CharEscapeIter::new([b'\\', b'n']),
                SINGLE_QUOTE => CharEscapeIter::new([b'\\', b'\'']),
                DOUBLE_QUOTE => CharEscapeIter::new([b'\\', b'"']),
                BACKSLASH => CharEscapeIter::new([b'\\', b'\\']),
                0x20..=0x7e => CharEscapeIter::new([self.as_u32() as u8]),
                _ => self.escape_unicode(),
            }
        }
    }

    /// See [`char::escape_unicode`].
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    /// assert_eq!(
    ///     "\\u{2764}",
    ///     JavaCodePoint::from_char('❤').escape_unicode().to_string()
    /// );
    /// assert_eq!(
    ///     "\\u{d800}",
    ///     JavaCodePoint::from_u32(0xd800)
    ///         .unwrap()
    ///         .escape_unicode()
    ///         .to_string()
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn escape_unicode(self) -> CharEscapeIter {
        let x = self.as_u32();

        let mut arr = [0; 10];
        arr[0] = b'\\';
        arr[1] = b'u';
        arr[2] = b'{';

        let number_len = if x == 0 {
            1
        } else {
            ((x.ilog2() >> 2) + 1) as usize
        };
        arr[3 + number_len] = b'}';
        for hexit in 0..number_len {
            arr[2 + number_len - hexit] = b"0123456789abcdef"[((x >> (hexit << 2)) & 15) as usize];
        }

        CharEscapeIter {
            inner: EscapeIterInner::Escaped(EscapeIterEscaped {
                bytes: arr,
                range: 0..number_len + 4,
            }),
        }
    }

    /// See [`char::is_alphabetic`].
    #[inline]
    #[must_use]
    pub fn is_alphabetic(self) -> bool {
        self.as_char().is_some_and(|char| char.is_alphabetic())
    }

    /// See [`char::is_alphanumeric`].
    #[inline]
    #[must_use]
    pub fn is_alphanumeric(self) -> bool {
        self.as_char().is_some_and(|char| char.is_alphanumeric())
    }

    /// See [`char::is_ascii`].
    #[inline]
    #[must_use]
    pub fn is_ascii(self) -> bool {
        self.as_u32() <= 0x7f
    }

    /// See [`char::is_ascii_alphabetic`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_alphabetic(self) -> bool {
        self.is_ascii_lowercase() || self.is_ascii_uppercase()
    }

    /// See [`char::is_ascii_alphanumeric`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_alphanumeric(self) -> bool {
        self.is_ascii_alphabetic() || self.is_ascii_digit()
    }

    /// See [`char::is_ascii_control`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_control(self) -> bool {
        matches!(self.as_u32(), 0..=0x1f | 0x7f)
    }

    /// See [`char::is_ascii_digit`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_digit(self) -> bool {
        const ZERO: u32 = '0' as u32;
        const NINE: u32 = '9' as u32;
        matches!(self.as_u32(), ZERO..=NINE)
    }

    /// See [`char::is_ascii_graphic`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_graphic(self) -> bool {
        matches!(self.as_u32(), 0x21..=0x7e)
    }

    /// See [`char::is_ascii_hexdigit`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_hexdigit(self) -> bool {
        const LOWER_A: u32 = 'a' as u32;
        const LOWER_F: u32 = 'f' as u32;
        const UPPER_A: u32 = 'A' as u32;
        const UPPER_F: u32 = 'F' as u32;
        self.is_ascii_digit() || matches!(self.as_u32(), (LOWER_A..=LOWER_F) | (UPPER_A..=UPPER_F))
    }

    /// See [`char::is_ascii_lowercase`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_lowercase(self) -> bool {
        const A: u32 = 'a' as u32;
        const Z: u32 = 'z' as u32;
        matches!(self.as_u32(), A..=Z)
    }

    /// See [`char::is_ascii_octdigit`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_octdigit(self) -> bool {
        const ZERO: u32 = '0' as u32;
        const SEVEN: u32 = '7' as u32;
        matches!(self.as_u32(), ZERO..=SEVEN)
    }

    /// See [`char::is_ascii_punctuation`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_punctuation(self) -> bool {
        matches!(
            self.as_u32(),
            (0x21..=0x2f) | (0x3a..=0x40) | (0x5b..=0x60) | (0x7b..=0x7e)
        )
    }

    /// See [`char::is_ascii_uppercase`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_uppercase(self) -> bool {
        const A: u32 = 'A' as u32;
        const Z: u32 = 'Z' as u32;
        matches!(self.as_u32(), A..=Z)
    }

    /// See [`char::is_ascii_whitespace`].
    #[inline]
    #[must_use]
    pub const fn is_ascii_whitespace(self) -> bool {
        const SPACE: u32 = ' ' as u32;
        const HORIZONTAL_TAB: u32 = '\t' as u32;
        const LINE_FEED: u32 = '\n' as u32;
        const FORM_FEED: u32 = 0xc;
        const CARRIAGE_RETURN: u32 = '\r' as u32;
        matches!(
            self.as_u32(),
            SPACE | HORIZONTAL_TAB | LINE_FEED | FORM_FEED | CARRIAGE_RETURN
        )
    }

    /// See [`char::is_control`].
    #[inline]
    #[must_use]
    pub fn is_control(self) -> bool {
        self.as_char().is_some_and(|char| char.is_control())
    }

    /// See [`char::is_digit`].
    #[inline]
    #[must_use]
    pub fn is_digit(self, radix: u32) -> bool {
        self.to_digit(radix).is_some()
    }

    /// See [`char::is_lowercase`].
    #[inline]
    #[must_use]
    pub fn is_lowercase(self) -> bool {
        self.as_char().is_some_and(|char| char.is_lowercase())
    }

    /// See [`char::is_numeric`].
    #[inline]
    #[must_use]
    pub fn is_numeric(self) -> bool {
        self.as_char().is_some_and(|char| char.is_numeric())
    }

    /// See [`char::is_uppercase`].
    #[inline]
    #[must_use]
    pub fn is_uppercase(self) -> bool {
        self.as_char().is_some_and(|char| char.is_uppercase())
    }

    /// See [`char::is_whitespace`].
    #[inline]
    #[must_use]
    pub fn is_whitespace(self) -> bool {
        self.as_char().is_some_and(|char| char.is_whitespace())
    }

    /// See [`char::len_utf16`]. Surrogate code points return 1.
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    ///
    /// let n = JavaCodePoint::from_char('ß').len_utf16();
    /// assert_eq!(n, 1);
    ///
    /// let len = JavaCodePoint::from_char('💣').len_utf16();
    /// assert_eq!(len, 2);
    ///
    /// assert_eq!(1, JavaCodePoint::from_u32(0xd800).unwrap().len_utf16());
    /// ```
    #[inline]
    #[must_use]
    pub const fn len_utf16(self) -> usize {
        if let Some(char) = self.as_char() {
            char.len_utf16()
        } else {
            1 // invalid code points are encoded as 1 utf16 code point anyway
        }
    }

    /// See [`char::len_utf8`]. Surrogate code points return 3.
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    ///
    /// let len = JavaCodePoint::from_char('A').len_utf8();
    /// assert_eq!(len, 1);
    ///
    /// let len = JavaCodePoint::from_char('ß').len_utf8();
    /// assert_eq!(len, 2);
    ///
    /// let len = JavaCodePoint::from_char('ℝ').len_utf8();
    /// assert_eq!(len, 3);
    ///
    /// let len = JavaCodePoint::from_char('💣').len_utf8();
    /// assert_eq!(len, 4);
    ///
    /// let len = JavaCodePoint::from_u32(0xd800).unwrap().len_utf8();
    /// assert_eq!(len, 3);
    /// ```
    #[inline]
    #[must_use]
    pub const fn len_utf8(self) -> usize {
        if let Some(char) = self.as_char() {
            char.len_utf8()
        } else {
            3 // invalid code points are all length 3 in semi-valid utf8
        }
    }

    /// See [`char::make_ascii_lowercase`].
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase();
    }

    /// See [`char::make_ascii_uppercase`].
    #[inline]
    pub fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase();
    }

    /// See [`char::to_ascii_lowercase`].
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    ///
    /// let ascii = JavaCodePoint::from_char('A');
    /// let non_ascii = JavaCodePoint::from_char('❤');
    ///
    /// assert_eq!('a', ascii.to_ascii_lowercase());
    /// assert_eq!('❤', non_ascii.to_ascii_lowercase());
    /// ```
    #[inline]
    #[must_use]
    pub const fn to_ascii_lowercase(self) -> JavaCodePoint {
        if self.is_ascii_uppercase() {
            unsafe {
                // SAFETY: all lowercase chars are valid chars
                Self::from_u32_unchecked(self.as_u32() + 32)
            }
        } else {
            self
        }
    }

    /// See [`char::to_ascii_uppercase`].
    ///
    /// ```
    /// # use java_string::JavaCodePoint;
    ///
    /// let ascii = JavaCodePoint::from_char('a');
    /// let non_ascii = JavaCodePoint::from_char('❤');
    ///
    /// assert_eq!('A', ascii.to_ascii_uppercase());
    /// assert_eq!('❤', non_ascii.to_ascii_uppercase());
    /// ```
    #[inline]
    #[must_use]
    pub const fn to_ascii_uppercase(self) -> JavaCodePoint {
        if self.is_ascii_lowercase() {
            unsafe {
                // SAFETY: all uppercase chars are valid chars
                Self::from_u32_unchecked(self.as_u32() - 32)
            }
        } else {
            self
        }
    }

    /// See [`char::to_digit`].
    #[inline]
    #[must_use]
    pub const fn to_digit(self, radix: u32) -> Option<u32> {
        if let Some(char) = self.as_char() {
            char.to_digit(radix)
        } else {
            None
        }
    }

    /// See [`char::to_lowercase`].
    #[inline]
    #[must_use]
    pub fn to_lowercase(self) -> ToLowercase {
        match self.as_char() {
            Some(char) => ToLowercase::char(char.to_lowercase()),
            None => ToLowercase::invalid(self),
        }
    }

    /// See [`char::to_uppercase`].
    #[inline]
    #[must_use]
    pub fn to_uppercase(self) -> ToUppercase {
        match self.as_char() {
            Some(char) => ToUppercase::char(char.to_uppercase()),
            None => ToUppercase::invalid(self),
        }
    }
}

impl Debug for JavaCodePoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_char('\'')?;
        for c in self.escape_debug_ext(EscapeDebugExtArgs {
            escape_single_quote: true,
            escape_double_quote: false,
        }) {
            f.write_char(c)?;
        }
        f.write_char('\'')
    }
}

impl Default for JavaCodePoint {
    #[inline]
    fn default() -> Self {
        JavaCodePoint::from_char('\0')
    }
}

impl Display for JavaCodePoint {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.as_char().unwrap_or(char::REPLACEMENT_CHARACTER), f)
    }
}

impl From<JavaCodePoint> for u32 {
    #[inline]
    fn from(value: JavaCodePoint) -> Self {
        value.as_u32()
    }
}

impl From<u8> for JavaCodePoint {
    #[inline]
    fn from(value: u8) -> Self {
        JavaCodePoint::from_char(char::from(value))
    }
}

impl FromStr for JavaCodePoint {
    type Err = ParseCharError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        char::from_str(s).map(JavaCodePoint::from_char)
    }
}

impl Hash for JavaCodePoint {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_u32().hash(state)
    }
}

impl Ord for JavaCodePoint {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_u32().cmp(&other.as_u32())
    }
}

impl PartialOrd for JavaCodePoint {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<char> for JavaCodePoint {
    #[inline]
    fn partial_cmp(&self, other: &char) -> Option<Ordering> {
        self.partial_cmp(&JavaCodePoint::from_char(*other))
    }
}

impl PartialOrd<JavaCodePoint> for char {
    #[inline]
    fn partial_cmp(&self, other: &JavaCodePoint) -> Option<Ordering> {
        JavaCodePoint::from_char(*self).partial_cmp(other)
    }
}

impl PartialEq<char> for JavaCodePoint {
    #[inline]
    fn eq(&self, other: &char) -> bool {
        self == &JavaCodePoint::from_char(*other)
    }
}

impl PartialEq<JavaCodePoint> for char {
    #[inline]
    fn eq(&self, other: &JavaCodePoint) -> bool {
        &JavaCodePoint::from_char(*self) == other
    }
}

pub(crate) struct EscapeDebugExtArgs {
    pub(crate) escape_single_quote: bool,
    pub(crate) escape_double_quote: bool,
}

impl EscapeDebugExtArgs {
    pub(crate) const ESCAPE_ALL: Self = Self {
        escape_single_quote: true,
        escape_double_quote: true,
    };
}

#[derive(Clone, Debug)]
pub struct CharEscapeIter {
    inner: EscapeIterInner,
}

#[derive(Clone, Debug)]
enum EscapeIterInner {
    Printable(Once<char>),
    Escaped(EscapeIterEscaped),
}

impl Display for EscapeIterInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EscapeIterInner::Printable(char) => char.clone().try_for_each(|ch| f.write_char(ch)),
            EscapeIterInner::Escaped(escaped) => Display::fmt(escaped, f),
        }
    }
}

impl CharEscapeIter {
    #[inline]
    fn printable(char: char) -> Self {
        CharEscapeIter {
            inner: EscapeIterInner::Printable(once(char)),
        }
    }

    /// # Safety
    /// Assumes that the input byte array is ASCII
    #[inline]
    unsafe fn new<const N: usize>(bytes: [u8; N]) -> Self {
        assert!(N <= 10, "Too many bytes in escape iter");
        let mut ten_bytes = [0; 10];
        ten_bytes[..N].copy_from_slice(&bytes);
        CharEscapeIter {
            inner: EscapeIterInner::Escaped(EscapeIterEscaped {
                bytes: ten_bytes,
                range: 0..N,
            }),
        }
    }
}

impl Iterator for CharEscapeIter {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            EscapeIterInner::Printable(printable) => printable.next(),
            EscapeIterInner::Escaped(escaped) => escaped.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            EscapeIterInner::Printable(printable) => printable.size_hint(),
            EscapeIterInner::Escaped(escaped) => escaped.size_hint(),
        }
    }
}

impl ExactSizeIterator for CharEscapeIter {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            EscapeIterInner::Printable(printable) => printable.len(),
            EscapeIterInner::Escaped(escaped) => escaped.len(),
        }
    }
}

impl FusedIterator for CharEscapeIter {}

impl Display for CharEscapeIter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[derive(Clone, Debug)]
struct EscapeIterEscaped {
    // SAFETY: all values must be in the ASCII range
    bytes: [u8; 10],
    // SAFETY: range must not be out of bounds for length 10
    range: Range<usize>,
}

impl Iterator for EscapeIterEscaped {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|index| unsafe {
            // SAFETY: the range is never out of bounds for length 10
            char::from(*self.bytes.get_unchecked(index))
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.range.len()
    }
}

impl ExactSizeIterator for EscapeIterEscaped {
    #[inline]
    fn len(&self) -> usize {
        self.range.len()
    }
}

impl FusedIterator for EscapeIterEscaped {}

impl Display for EscapeIterEscaped {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = unsafe {
            // SAFETY: all bytes are in ASCII range, and range is in bounds for length 10
            std::str::from_utf8_unchecked(self.bytes.get_unchecked(self.range.clone()))
        };
        f.write_str(str)
    }
}

pub type ToLowercase = CharIterDelegate<std::char::ToLowercase>;
pub type ToUppercase = CharIterDelegate<std::char::ToUppercase>;

#[derive(Debug, Clone)]
pub struct CharIterDelegate<I>(CharIterDelegateInner<I>);

impl<I> CharIterDelegate<I> {
    #[inline]
    fn char(iter: I) -> CharIterDelegate<I> {
        CharIterDelegate(CharIterDelegateInner::Char(iter))
    }

    #[inline]
    fn invalid(code_point: JavaCodePoint) -> CharIterDelegate<I> {
        CharIterDelegate(CharIterDelegateInner::Invalid(Some(code_point).into_iter()))
    }
}

#[derive(Debug, Clone)]
enum CharIterDelegateInner<I> {
    Char(I),
    Invalid(std::option::IntoIter<JavaCodePoint>),
}

impl<I> Iterator for CharIterDelegate<I>
where
    I: Iterator<Item = char>,
{
    type Item = JavaCodePoint;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            CharIterDelegateInner::Char(char_iter) => {
                char_iter.next().map(JavaCodePoint::from_char)
            }
            CharIterDelegateInner::Invalid(code_point) => code_point.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            CharIterDelegateInner::Char(char_iter) => char_iter.size_hint(),
            CharIterDelegateInner::Invalid(code_point) => code_point.size_hint(),
        }
    }
}

impl<I> DoubleEndedIterator for CharIterDelegate<I>
where
    I: Iterator<Item = char> + DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            CharIterDelegateInner::Char(char_iter) => {
                char_iter.next_back().map(JavaCodePoint::from_char)
            }
            CharIterDelegateInner::Invalid(code_point) => code_point.next_back(),
        }
    }
}

impl<I> ExactSizeIterator for CharIterDelegate<I> where I: Iterator<Item = char> + ExactSizeIterator {}

impl<I> FusedIterator for CharIterDelegate<I> where I: Iterator<Item = char> + FusedIterator {}
