use std::borrow::{Borrow, BorrowMut, Cow};
use std::collections::{Bound, TryReserveError};
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::FusedIterator;
use std::ops::{
    Add, AddAssign, Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull,
    RangeInclusive, RangeTo, RangeToInclusive,
};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::{ptr, slice};

use crate::validations::{
    run_utf8_full_validation_from_semi, run_utf8_semi_validation, to_range_checked,
};
use crate::{Chars, FromUtf8Error, JavaCodePoint, JavaStr, Utf8Error};

#[derive(Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct JavaString {
    vec: Vec<u8>,
}

impl JavaString {
    #[inline]
    #[must_use]
    pub const fn new() -> JavaString {
        JavaString { vec: Vec::new() }
    }

    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> JavaString {
        JavaString {
            vec: Vec::with_capacity(capacity),
        }
    }

    /// Converts `vec` to a `JavaString` if it is fully-valid UTF-8, i.e. UTF-8
    /// without surrogate code points. See [`String::from_utf8`].
    #[inline]
    pub fn from_full_utf8(vec: Vec<u8>) -> Result<JavaString, FromUtf8Error> {
        match std::str::from_utf8(&vec) {
            Ok(..) => Ok(JavaString { vec }),
            Err(e) => Err(FromUtf8Error {
                bytes: vec,
                error: e.into(),
            }),
        }
    }

    /// Converts `vec` to a `JavaString` if it is semi-valid UTF-8, i.e. UTF-8
    /// with surrogate code points.
    ///
    /// ```
    /// # use java_string::{JavaCodePoint, JavaString};
    ///
    /// assert_eq!(
    ///     JavaString::from_semi_utf8(b"Hello World!".to_vec()).unwrap(),
    ///     "Hello World!"
    /// );
    /// assert_eq!(
    ///     JavaString::from_semi_utf8(vec![0xf0, 0x9f, 0x92, 0x96]).unwrap(),
    ///     "💖"
    /// );
    /// assert_eq!(
    ///     JavaString::from_semi_utf8(vec![0xed, 0xa0, 0x80]).unwrap(),
    ///     JavaString::from(JavaCodePoint::from_u32(0xd800).unwrap())
    /// );
    /// assert!(JavaString::from_semi_utf8(vec![0xed]).is_err());
    /// ```
    pub fn from_semi_utf8(vec: Vec<u8>) -> Result<JavaString, FromUtf8Error> {
        match run_utf8_semi_validation(&vec) {
            Ok(..) => Ok(JavaString { vec }),
            Err(err) => Err(FromUtf8Error {
                bytes: vec,
                error: err,
            }),
        }
    }

    /// Converts `v` to a `Cow<JavaStr>`, replacing invalid semi-UTF-8 with the
    /// replacement character �.
    ///
    /// ```
    /// # use std::borrow::Cow;
    /// # use java_string::{JavaStr, JavaString};
    ///
    /// let sparkle_heart = [0xf0, 0x9f, 0x92, 0x96];
    /// let result = JavaString::from_semi_utf8_lossy(&sparkle_heart);
    /// assert!(matches!(result, Cow::Borrowed(_)));
    /// assert_eq!(result, JavaStr::from_str("💖"));
    ///
    /// let foobar_with_error = [b'f', b'o', b'o', 0xed, b'b', b'a', b'r'];
    /// let result = JavaString::from_semi_utf8_lossy(&foobar_with_error);
    /// assert!(matches!(result, Cow::Owned(_)));
    /// assert_eq!(result, JavaStr::from_str("foo�bar"));
    /// ```
    #[must_use]
    pub fn from_semi_utf8_lossy(v: &[u8]) -> Cow<'_, JavaStr> {
        const REPLACEMENT: &str = "\u{FFFD}";

        match run_utf8_semi_validation(v) {
            Ok(()) => unsafe {
                // SAFETY: validation succeeded
                Cow::Borrowed(JavaStr::from_semi_utf8_unchecked(v))
            },
            Err(error) => {
                let mut result = unsafe {
                    // SAFETY: validation succeeded up to this index
                    JavaString::from_semi_utf8_unchecked(
                        v.get_unchecked(..error.valid_up_to).to_vec(),
                    )
                };
                result.push_str(REPLACEMENT);
                let mut index = error.valid_up_to + error.error_len.unwrap_or(1) as usize;
                loop {
                    match run_utf8_semi_validation(&v[index..]) {
                        Ok(()) => {
                            unsafe {
                                // SAFETY: validation succeeded
                                result
                                    .push_java_str(JavaStr::from_semi_utf8_unchecked(&v[index..]));
                            }
                            return Cow::Owned(result);
                        }
                        Err(error) => {
                            unsafe {
                                // SAFETY: validation succeeded up to this index
                                result.push_java_str(JavaStr::from_semi_utf8_unchecked(
                                    v.get_unchecked(index..index + error.valid_up_to),
                                ));
                            }
                            result.push_str(REPLACEMENT);
                            index += error.valid_up_to + error.error_len.unwrap_or(1) as usize;
                        }
                    }
                }
            }
        }
    }

    /// # Safety
    ///
    /// The parameter must be in semi-valid UTF-8 format, that is, UTF-8 plus
    /// surrogate code points.
    #[inline]
    #[must_use]
    pub unsafe fn from_semi_utf8_unchecked(bytes: Vec<u8>) -> JavaString {
        JavaString { vec: bytes }
    }

    /// See [`String::into_bytes`].
    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.vec
    }

    /// See [`String::as_str`].
    #[inline]
    #[must_use]
    pub fn as_java_str(&self) -> &JavaStr {
        unsafe {
            // SAFETY: this str has semi-valid UTF-8
            JavaStr::from_semi_utf8_unchecked(&self.vec)
        }
    }

    /// See [`String::as_mut_str`].
    #[inline]
    #[must_use]
    pub fn as_mut_java_str(&mut self) -> &mut JavaStr {
        unsafe {
            // SAFETY: this str has semi-valid UTF-8
            JavaStr::from_semi_utf8_unchecked_mut(&mut self.vec)
        }
    }

    /// Tries to convert this `JavaString` to a `String`, returning an error if
    /// it is not fully valid UTF-8, i.e. has no surrogate code points.
    ///
    /// ```
    /// # use java_string::{JavaCodePoint, JavaString};
    ///
    /// assert_eq!(
    ///     JavaString::from("Hello World!").into_string().unwrap(),
    ///     "Hello World!"
    /// );
    /// assert_eq!(
    ///     JavaString::from("abc\0ℝ💣").into_string().unwrap(),
    ///     "abc\0ℝ💣"
    /// );
    ///
    /// let string_with_error = JavaString::from("abc")
    ///     + JavaString::from(JavaCodePoint::from_u32(0xd800).unwrap()).as_java_str();
    /// assert!(string_with_error.into_string().is_err());
    /// ```
    pub fn into_string(self) -> Result<String, Utf8Error> {
        run_utf8_full_validation_from_semi(self.as_bytes()).map(|()| unsafe {
            // SAFETY: validation succeeded
            self.into_string_unchecked()
        })
    }

    /// # Safety
    ///
    /// This string must be fully valid UTF-8, i.e. have no surrogate code
    /// points.
    #[inline]
    #[must_use]
    pub unsafe fn into_string_unchecked(self) -> String {
        // SAFETY: preconditions checked by caller
        String::from_utf8_unchecked(self.vec)
    }

    /// See [`String::push_str`].
    #[inline]
    pub fn push_java_str(&mut self, string: &JavaStr) {
        self.vec.extend_from_slice(string.as_bytes())
    }

    /// See [`String::push_str`].
    #[inline]
    pub fn push_str(&mut self, string: &str) {
        self.vec.extend_from_slice(string.as_bytes())
    }

    /// See [`String::capacity`].
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// See [`String::reserve`].
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional)
    }

    /// See [`String::reserve_exact`].
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.vec.reserve_exact(additional)
    }

    /// See [`String::try_reserve`].
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.vec.try_reserve(additional)
    }

    /// See [`String::try_reserve_exact`].
    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.vec.try_reserve_exact(additional)
    }

    /// See [`String::shrink_to_fit`].
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }

    /// See [`String::shrink_to`].
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.vec.shrink_to(min_capacity)
    }

    /// See [`String::push`].
    #[inline]
    pub fn push(&mut self, ch: char) {
        match ch.len_utf8() {
            1 => self.vec.push(ch as u8),
            _ => self
                .vec
                .extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }

    /// See [`String::push`].
    #[inline]
    pub fn push_java(&mut self, ch: JavaCodePoint) {
        match ch.len_utf8() {
            1 => self.vec.push(ch.as_u32() as u8),
            _ => self.vec.extend_from_slice(ch.encode_semi_utf8(&mut [0; 4])),
        }
    }

    /// See [`String::as_bytes`].
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.vec
    }

    /// See [`String::truncate`].
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len() {
            assert!(self.is_char_boundary(new_len));
            self.vec.truncate(new_len)
        }
    }

    /// See [`String::pop`].
    ///
    /// ```
    /// # use java_string::JavaString;
    ///
    /// let mut str = JavaString::from("Hello World!");
    /// assert_eq!(str.pop().unwrap(), '!');
    /// assert_eq!(str, "Hello World");
    ///
    /// let mut str = JavaString::from("東京");
    /// assert_eq!(str.pop().unwrap(), '京');
    /// assert_eq!(str, "東");
    ///
    /// assert!(JavaString::new().pop().is_none());
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<JavaCodePoint> {
        let ch = self.chars().next_back()?;
        let newlen = self.len() - ch.len_utf8();
        unsafe {
            self.vec.set_len(newlen);
        }
        Some(ch)
    }

    /// See [`String::remove`].
    ///
    /// ```
    /// # use java_string::JavaString;
    ///
    /// let mut str = JavaString::from("Hello World!");
    /// assert_eq!(str.remove(5), ' ');
    /// assert_eq!(str, "HelloWorld!");
    ///
    /// let mut str = JavaString::from("Hello 🦀 World!");
    /// assert_eq!(str.remove(6), '🦀');
    /// assert_eq!(str, "Hello  World!");
    /// ```
    /// ```should_panic
    /// # use java_string::JavaString;
    /// // Should panic
    /// JavaString::new().remove(0);
    /// ```
    /// ```should_panic
    /// # use java_string::JavaString;
    /// // Should panic
    /// JavaString::from("🦀").remove(1);
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> JavaCodePoint {
        let Some(ch) = self[idx..].chars().next() else {
            panic!("cannot remove a char from the end of a string")
        };

        let next = idx + ch.len_utf8();
        let len = self.len();
        unsafe {
            ptr::copy(
                self.vec.as_ptr().add(next),
                self.vec.as_mut_ptr().add(idx),
                len - next,
            );
            self.vec.set_len(len - (next - idx));
        }
        ch
    }

    /// See [`String::retain`].
    ///
    /// ```
    /// # use java_string::{JavaCodePoint, JavaString};
    ///
    /// let mut str = JavaString::from("Hello 🦀 World!");
    /// str.retain(|ch| !ch.is_ascii_uppercase());
    /// assert_eq!(str, "ello 🦀 orld!");
    /// str.retain(JavaCodePoint::is_ascii);
    /// assert_eq!(str, "ello  orld!");
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(JavaCodePoint) -> bool,
    {
        struct SetLenOnDrop<'a> {
            s: &'a mut JavaString,
            idx: usize,
            del_bytes: usize,
        }

        impl<'a> Drop for SetLenOnDrop<'a> {
            #[inline]
            fn drop(&mut self) {
                let new_len = self.idx - self.del_bytes;
                debug_assert!(new_len <= self.s.len());
                unsafe { self.s.vec.set_len(new_len) };
            }
        }

        let len = self.len();
        let mut guard = SetLenOnDrop {
            s: self,
            idx: 0,
            del_bytes: 0,
        };

        while guard.idx < len {
            // SAFETY: `guard.idx` is positive-or-zero and less that len so the
            // `get_unchecked` is in bound. `self` is valid UTF-8 like string
            // and the returned slice starts at a unicode code point so the
            // `Chars` always return one character.
            let ch = unsafe {
                guard
                    .s
                    .get_unchecked(guard.idx..len)
                    .chars()
                    .next()
                    .unwrap_unchecked()
            };
            let ch_len = ch.len_utf8();

            if !f(ch) {
                guard.del_bytes += ch_len;
            } else if guard.del_bytes > 0 {
                // SAFETY: `guard.idx` is in bound and `guard.del_bytes` represent the number of
                // bytes that are erased from the string so the resulting `guard.idx -
                // guard.del_bytes` always represent a valid unicode code point.
                //
                // `guard.del_bytes` >= `ch.len_utf8()`, so taking a slice with `ch.len_utf8()`
                // len is safe.
                ch.encode_semi_utf8(unsafe {
                    slice::from_raw_parts_mut(
                        guard.s.as_mut_ptr().add(guard.idx - guard.del_bytes),
                        ch.len_utf8(),
                    )
                });
            }

            // Point idx to the next char
            guard.idx += ch_len;
        }

        drop(guard);
    }

    /// See [`String::insert`].
    ///
    /// ```
    /// # use java_string::JavaString;
    /// let mut s = JavaString::from("foo");
    /// s.insert(3, 'a');
    /// s.insert(4, 'r');
    /// s.insert(3, 'b');
    /// assert_eq!(s, "foobar");
    /// ```
    #[inline]
    pub fn insert(&mut self, idx: usize, ch: char) {
        assert!(self.is_char_boundary(idx));
        let mut bits = [0; 4];
        let bits = ch.encode_utf8(&mut bits).as_bytes();

        unsafe {
            self.insert_bytes(idx, bits);
        }
    }

    /// See [`String::insert`].
    #[inline]
    pub fn insert_java(&mut self, idx: usize, ch: JavaCodePoint) {
        assert!(self.is_char_boundary(idx));
        let mut bits = [0; 4];
        let bits = ch.encode_semi_utf8(&mut bits);

        unsafe {
            self.insert_bytes(idx, bits);
        }
    }

    #[inline]
    unsafe fn insert_bytes(&mut self, idx: usize, bytes: &[u8]) {
        let len = self.len();
        let amt = bytes.len();
        self.vec.reserve(amt);

        unsafe {
            ptr::copy(
                self.vec.as_ptr().add(idx),
                self.vec.as_mut_ptr().add(idx + amt),
                len - idx,
            );
            ptr::copy_nonoverlapping(bytes.as_ptr(), self.vec.as_mut_ptr().add(idx), amt);
            self.vec.set_len(len + amt);
        }
    }

    /// See [`String::insert_str`].
    ///
    /// ```
    /// # use java_string::JavaString;
    /// let mut s = JavaString::from("bar");
    /// s.insert_str(0, "foo");
    /// assert_eq!(s, "foobar");
    /// ```
    #[inline]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        assert!(self.is_char_boundary(idx));

        unsafe {
            self.insert_bytes(idx, string.as_bytes());
        }
    }

    /// See [`String::insert_str`].
    pub fn insert_java_str(&mut self, idx: usize, string: &JavaStr) {
        assert!(self.is_char_boundary(idx));

        unsafe {
            self.insert_bytes(idx, string.as_bytes());
        }
    }

    /// See [`String::as_mut_vec`].
    ///
    /// # Safety
    ///
    /// The returned `Vec` must not have invalid UTF-8 written to it, besides
    /// surrogate pairs.
    #[inline]
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.vec
    }

    /// See [`String::len`].
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// See [`String::is_empty`].
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// See [`String::split_off`].
    ///
    /// ```
    /// # use java_string::JavaString;
    /// let mut hello = JavaString::from("Hello World!");
    /// let world = hello.split_off(6);
    /// assert_eq!(hello, "Hello ");
    /// assert_eq!(world, "World!");
    /// ```
    /// ```should_panic
    /// # use java_string::JavaString;
    /// let mut s = JavaString::from("🦀");
    /// // Should panic
    /// let _ = s.split_off(1);
    /// ```
    #[inline]
    #[must_use]
    pub fn split_off(&mut self, at: usize) -> JavaString {
        assert!(self.is_char_boundary(at));
        let other = self.vec.split_off(at);
        unsafe { JavaString::from_semi_utf8_unchecked(other) }
    }

    /// See [`String::clear`].
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear();
    }

    /// See [`String::drain`].
    ///
    /// ```
    /// # use java_string::JavaString;
    ///
    /// let mut s = JavaString::from("α is alpha, β is beta");
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Remove the range up until the β from the string
    /// let t: JavaString = s.drain(..beta_offset).collect();
    /// assert_eq!(t, "α is alpha, ");
    /// assert_eq!(s, "β is beta");
    ///
    /// // A full range clears the string, like `clear()` does
    /// s.drain(..);
    /// assert_eq!(s, "");
    /// ```
    #[inline]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_>
    where
        R: RangeBounds<usize>,
    {
        // Memory safety: see String::drain
        let Range { start, end } = to_range_checked(range, ..self.len());
        assert!(self.is_char_boundary(start));
        assert!(self.is_char_boundary(end));

        // Take out two simultaneous borrows. The &mut String won't be accessed
        // until iteration is over, in Drop.
        let self_ptr = self as *mut _;
        // SAFETY: `to_range_checked` and `is_char_boundary` do the appropriate bounds
        // checks.
        let chars_iter = unsafe { self.get_unchecked(start..end) }.chars();

        Drain {
            start,
            end,
            iter: chars_iter,
            string: self_ptr,
        }
    }

    /// See [`String::replace_range`].
    ///
    /// ```
    /// # use java_string::JavaString;
    ///
    /// let mut s = JavaString::from("α is alpha, β is beta");
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Replace the range up until the β from the string
    /// s.replace_range(..beta_offset, "Α is capital alpha; ");
    /// assert_eq!(s, "Α is capital alpha; β is beta");
    /// ```
    /// ```should_panic
    /// # use java_string::JavaString;
    /// let mut s = JavaString::from("α is alpha, β is beta");
    /// // Should panic
    /// s.replace_range(..1, "Α is capital alpha; ");
    /// ```
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        self.replace_range_java(range, JavaStr::from_str(replace_with))
    }

    /// See [`String::replace_range`].
    pub fn replace_range_java<R>(&mut self, range: R, replace_with: &JavaStr)
    where
        R: RangeBounds<usize>,
    {
        let start = range.start_bound();
        match start {
            Bound::Included(&n) => assert!(self.is_char_boundary(n)),
            Bound::Excluded(&n) => assert!(self.is_char_boundary(n + 1)),
            Bound::Unbounded => {}
        };
        let end = range.end_bound();
        match end {
            Bound::Included(&n) => assert!(self.is_char_boundary(n + 1)),
            Bound::Excluded(&n) => assert!(self.is_char_boundary(n)),
            Bound::Unbounded => {}
        };

        unsafe { self.as_mut_vec() }.splice((start, end), replace_with.bytes());
    }

    /// See [`String::into_boxed_str`].
    #[inline]
    #[must_use]
    pub fn into_boxed_str(self) -> Box<JavaStr> {
        let slice = self.vec.into_boxed_slice();
        unsafe { JavaStr::from_boxed_semi_utf8_unchecked(slice) }
    }

    /// See [`String::leak`].
    #[inline]
    pub fn leak<'a>(self) -> &'a mut JavaStr {
        let slice = self.vec.leak();
        unsafe { JavaStr::from_semi_utf8_unchecked_mut(slice) }
    }
}

impl Add<&str> for JavaString {
    type Output = JavaString;

    #[inline]
    fn add(mut self, rhs: &str) -> Self::Output {
        self.push_str(rhs);
        self
    }
}

impl Add<&JavaStr> for JavaString {
    type Output = JavaString;

    #[inline]
    fn add(mut self, rhs: &JavaStr) -> Self::Output {
        self.push_java_str(rhs);
        self
    }
}

impl AddAssign<&str> for JavaString {
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl AddAssign<&JavaStr> for JavaString {
    #[inline]
    fn add_assign(&mut self, rhs: &JavaStr) {
        self.push_java_str(rhs);
    }
}

impl AsMut<JavaStr> for JavaString {
    #[inline]
    fn as_mut(&mut self) -> &mut JavaStr {
        self.as_mut_java_str()
    }
}

impl AsRef<[u8]> for JavaString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<JavaStr> for JavaString {
    #[inline]
    fn as_ref(&self) -> &JavaStr {
        self.as_java_str()
    }
}

impl Borrow<JavaStr> for JavaString {
    #[inline]
    fn borrow(&self) -> &JavaStr {
        self.as_java_str()
    }
}

impl BorrowMut<JavaStr> for JavaString {
    #[inline]
    fn borrow_mut(&mut self) -> &mut JavaStr {
        self.as_mut_java_str()
    }
}

impl Clone for JavaString {
    #[inline]
    fn clone(&self) -> Self {
        JavaString {
            vec: self.vec.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.vec.clone_from(&source.vec)
    }
}

impl Debug for JavaString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl Deref for JavaString {
    type Target = JavaStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_java_str()
    }
}

impl DerefMut for JavaString {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_java_str()
    }
}

impl Display for JavaString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl Extend<char> for JavaString {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        let iterator = iter.into_iter();
        let (lower_bound, _) = iterator.size_hint();
        self.reserve(lower_bound);
        iterator.for_each(move |c| self.push(c));
    }
}

impl Extend<JavaCodePoint> for JavaString {
    fn extend<T: IntoIterator<Item = JavaCodePoint>>(&mut self, iter: T) {
        let iterator = iter.into_iter();
        let (lower_bound, _) = iterator.size_hint();
        self.reserve(lower_bound);
        iterator.for_each(move |c| self.push_java(c));
    }
}

impl Extend<String> for JavaString {
    fn extend<T: IntoIterator<Item = String>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl Extend<JavaString> for JavaString {
    fn extend<T: IntoIterator<Item = JavaString>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_java_str(&s));
    }
}

impl<'a> Extend<&'a char> for JavaString {
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        self.extend(iter.into_iter().copied())
    }
}

impl<'a> Extend<&'a JavaCodePoint> for JavaString {
    fn extend<T: IntoIterator<Item = &'a JavaCodePoint>>(&mut self, iter: T) {
        self.extend(iter.into_iter().copied())
    }
}

impl<'a> Extend<&'a str> for JavaString {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(s));
    }
}

impl<'a> Extend<&'a JavaStr> for JavaString {
    fn extend<T: IntoIterator<Item = &'a JavaStr>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_java_str(s));
    }
}

impl Extend<Box<str>> for JavaString {
    fn extend<T: IntoIterator<Item = Box<str>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl Extend<Box<JavaStr>> for JavaString {
    fn extend<T: IntoIterator<Item = Box<JavaStr>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_java_str(&s));
    }
}

impl<'a> Extend<Cow<'a, str>> for JavaString {
    fn extend<T: IntoIterator<Item = Cow<'a, str>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl<'a> Extend<Cow<'a, JavaStr>> for JavaString {
    fn extend<T: IntoIterator<Item = Cow<'a, JavaStr>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_java_str(&s));
    }
}

impl From<String> for JavaString {
    #[inline]
    fn from(value: String) -> Self {
        unsafe {
            // SAFETY: value is valid UTF-8
            JavaString::from_semi_utf8_unchecked(value.into_bytes())
        }
    }
}

impl From<&String> for JavaString {
    #[inline]
    fn from(value: &String) -> Self {
        Self::from(value.clone())
    }
}

impl From<&JavaString> for JavaString {
    #[inline]
    fn from(value: &JavaString) -> Self {
        value.clone()
    }
}

impl From<&mut str> for JavaString {
    #[inline]
    fn from(value: &mut str) -> Self {
        Self::from(&*value)
    }
}

impl From<&str> for JavaString {
    #[inline]
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<&mut JavaStr> for JavaString {
    #[inline]
    fn from(value: &mut JavaStr) -> Self {
        Self::from(&*value)
    }
}

impl From<&JavaStr> for JavaString {
    #[inline]
    fn from(value: &JavaStr) -> Self {
        value.to_owned()
    }
}

impl From<Box<str>> for JavaString {
    #[inline]
    fn from(value: Box<str>) -> Self {
        Self::from(value.into_string())
    }
}

impl From<Box<JavaStr>> for JavaString {
    #[inline]
    fn from(value: Box<JavaStr>) -> Self {
        value.into_string()
    }
}

impl<'a> From<Cow<'a, str>> for JavaString {
    #[inline]
    fn from(value: Cow<'a, str>) -> Self {
        Self::from(value.into_owned())
    }
}

impl<'a> From<Cow<'a, JavaStr>> for JavaString {
    #[inline]
    fn from(value: Cow<'a, JavaStr>) -> Self {
        value.into_owned()
    }
}

impl From<JavaString> for Arc<JavaStr> {
    #[inline]
    fn from(value: JavaString) -> Self {
        Arc::from(&value[..])
    }
}

impl<'a> From<JavaString> for Cow<'a, JavaStr> {
    #[inline]
    fn from(value: JavaString) -> Self {
        Cow::Owned(value)
    }
}

impl From<JavaString> for Rc<JavaStr> {
    #[inline]
    fn from(value: JavaString) -> Self {
        Rc::from(&value[..])
    }
}

impl From<JavaString> for Vec<u8> {
    #[inline]
    fn from(value: JavaString) -> Self {
        value.into_bytes()
    }
}

impl From<char> for JavaString {
    #[inline]
    fn from(value: char) -> Self {
        Self::from(value.encode_utf8(&mut [0; 4]))
    }
}

impl From<JavaCodePoint> for JavaString {
    #[inline]
    fn from(value: JavaCodePoint) -> Self {
        unsafe {
            // SAFETY: we're encoding into semi-valid UTF-8
            JavaString::from_semi_utf8_unchecked(value.encode_semi_utf8(&mut [0; 4]).to_vec())
        }
    }
}

impl FromIterator<char> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a char> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<JavaCodePoint> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = JavaCodePoint>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a JavaCodePoint> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = &'a JavaCodePoint>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a str> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<String> for JavaString {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut iterator = iter.into_iter();

        match iterator.next() {
            None => JavaString::new(),
            Some(buf) => {
                let mut buf = JavaString::from(buf);
                buf.extend(iterator);
                buf
            }
        }
    }
}

impl FromIterator<JavaString> for JavaString {
    fn from_iter<T: IntoIterator<Item = JavaString>>(iter: T) -> Self {
        let mut iterator = iter.into_iter();

        match iterator.next() {
            None => JavaString::new(),
            Some(mut buf) => {
                buf.extend(iterator);
                buf
            }
        }
    }
}

impl FromIterator<Box<str>> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Box<str>>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<Box<JavaStr>> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Box<JavaStr>>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<Cow<'a, str>> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Cow<'a, str>>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<Cow<'a, JavaStr>> for JavaString {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Cow<'a, JavaStr>>>(iter: T) -> Self {
        let mut buf = JavaString::new();
        buf.extend(iter);
        buf
    }
}

impl FromStr for JavaString {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl Hash for JavaString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl Index<Range<usize>> for JavaString {
    type Output = JavaStr;

    #[inline]
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl Index<RangeFrom<usize>> for JavaString {
    type Output = JavaStr;

    #[inline]
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl Index<RangeFull> for JavaString {
    type Output = JavaStr;

    #[inline]
    fn index(&self, _index: RangeFull) -> &Self::Output {
        self.as_java_str()
    }
}

impl Index<RangeInclusive<usize>> for JavaString {
    type Output = JavaStr;

    #[inline]
    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl Index<RangeTo<usize>> for JavaString {
    type Output = JavaStr;

    #[inline]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl Index<RangeToInclusive<usize>> for JavaString {
    type Output = JavaStr;

    #[inline]
    fn index(&self, index: RangeToInclusive<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl IndexMut<Range<usize>> for JavaString {
    #[inline]
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl IndexMut<RangeFrom<usize>> for JavaString {
    #[inline]
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl IndexMut<RangeFull> for JavaString {
    #[inline]
    fn index_mut(&mut self, _index: RangeFull) -> &mut Self::Output {
        self.as_mut_java_str()
    }
}

impl IndexMut<RangeInclusive<usize>> for JavaString {
    #[inline]
    fn index_mut(&mut self, index: RangeInclusive<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl IndexMut<RangeTo<usize>> for JavaString {
    #[inline]
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl IndexMut<RangeToInclusive<usize>> for JavaString {
    #[inline]
    fn index_mut(&mut self, index: RangeToInclusive<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl PartialEq<str> for JavaString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self[..] == other
    }
}

impl PartialEq<JavaString> for str {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        self == other[..]
    }
}

impl<'a> PartialEq<&'a str> for JavaString {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<JavaString> for &'a str {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        *self == other
    }
}

impl PartialEq<String> for JavaString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        &self[..] == other
    }
}

impl PartialEq<JavaString> for String {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        self == &other[..]
    }
}

impl PartialEq<JavaStr> for JavaString {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        self[..] == other
    }
}

impl<'a> PartialEq<&'a JavaStr> for JavaString {
    #[inline]
    fn eq(&self, other: &&'a JavaStr) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<Cow<'a, str>> for JavaString {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        &self[..] == other
    }
}

impl<'a> PartialEq<JavaString> for Cow<'a, str> {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        self == &other[..]
    }
}

impl<'a> PartialEq<Cow<'a, JavaStr>> for JavaString {
    #[inline]
    fn eq(&self, other: &Cow<'a, JavaStr>) -> bool {
        &self[..] == other
    }
}

impl<'a> PartialEq<JavaString> for Cow<'a, JavaStr> {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        self == &other[..]
    }
}

impl Write for JavaString {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.push(c);
        Ok(())
    }
}

pub struct Drain<'a> {
    string: *mut JavaString,
    start: usize,
    end: usize,
    iter: Chars<'a>,
}

impl Debug for Drain<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Drain").field(&self.as_str()).finish()
    }
}

unsafe impl Sync for Drain<'_> {}
unsafe impl Send for Drain<'_> {}

impl Drop for Drain<'_> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            // Use Vec::drain. "Reaffirm" the bounds checks to avoid
            // panic code being inserted again.
            let self_vec = (*self.string).as_mut_vec();
            if self.start <= self.end && self.end <= self_vec.len() {
                self_vec.drain(self.start..self.end);
            }
        }
    }
}

impl AsRef<JavaStr> for Drain<'_> {
    #[inline]
    fn as_ref(&self) -> &JavaStr {
        self.as_str()
    }
}

impl AsRef<[u8]> for Drain<'_> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl Drain<'_> {
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &JavaStr {
        self.iter.as_str()
    }
}

impl Iterator for Drain<'_> {
    type Item = JavaCodePoint;

    #[inline]
    fn next(&mut self) -> Option<JavaCodePoint> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn last(mut self) -> Option<JavaCodePoint> {
        self.next_back()
    }
}

impl DoubleEndedIterator for Drain<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl FusedIterator for Drain<'_> {}
