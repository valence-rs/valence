use std::borrow::Cow;
use std::collections::Bound;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::ops::{
    Add, AddAssign, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive,
    RangeTo, RangeToInclusive,
};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::{ptr, slice};

use crate::char::EscapeDebugExtArgs;
use crate::validations::{
    run_utf8_full_validation_from_semi, run_utf8_semi_validation, slice_error_fail,
    str_end_index_overflow_fail,
};
use crate::{
    Bytes, CharEscapeIter, CharIndices, Chars, EscapeDebug, EscapeDefault, EscapeUnicode,
    JavaCodePoint, JavaStrPattern, JavaString, Lines, MatchIndices, Matches, ParseError,
    RMatchIndices, RMatches, RSplit, RSplitN, RSplitTerminator, Split, SplitAsciiWhitespace,
    SplitInclusive, SplitN, SplitTerminator, SplitWhitespace, Utf8Error,
};

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct JavaStr {
    inner: [u8],
}

impl JavaStr {
    #[inline]
    pub const fn from_full_utf8(v: &[u8]) -> Result<&JavaStr, Utf8Error> {
        match std::str::from_utf8(v) {
            Ok(str) => Ok(JavaStr::from_str(str)),
            Err(err) => Err(Utf8Error::from_std(err)),
        }
    }

    #[inline]
    pub fn from_full_utf8_mut(v: &mut [u8]) -> Result<&mut JavaStr, Utf8Error> {
        match std::str::from_utf8_mut(v) {
            Ok(str) => Ok(JavaStr::from_mut_str(str)),
            Err(err) => Err(Utf8Error::from_std(err)),
        }
    }

    pub fn from_semi_utf8(v: &[u8]) -> Result<&JavaStr, Utf8Error> {
        match run_utf8_semi_validation(v) {
            Ok(()) => Ok(unsafe { JavaStr::from_semi_utf8_unchecked(v) }),
            Err(err) => Err(err),
        }
    }

    pub fn from_semi_utf8_mut(v: &mut [u8]) -> Result<&mut JavaStr, Utf8Error> {
        match run_utf8_semi_validation(v) {
            Ok(()) => Ok(unsafe { JavaStr::from_semi_utf8_unchecked_mut(v) }),
            Err(err) => Err(err),
        }
    }

    /// # Safety
    ///
    /// The parameter must be in semi-valid UTF-8 format, that is, UTF-8 plus
    /// surrogate code points.
    #[inline]
    #[must_use]
    pub const unsafe fn from_semi_utf8_unchecked(v: &[u8]) -> &JavaStr {
        // SAFETY: the caller must guarantee that the bytes `v` are valid UTF-8, minus
        // the absence of surrogate chars. Also relies on `&JavaStr` and `&[u8]`
        // having the same layout.
        std::mem::transmute(v)
    }

    /// # Safety
    ///
    /// The parameter must be in semi-valid UTF-8 format, that is, UTF-8 plus
    /// surrogate code points.
    #[inline]
    #[must_use]
    pub unsafe fn from_semi_utf8_unchecked_mut(v: &mut [u8]) -> &mut JavaStr {
        // SAFETY: see from_semi_utf8_unchecked
        std::mem::transmute(v)
    }

    #[inline]
    #[must_use]
    pub const fn from_str(str: &str) -> &JavaStr {
        unsafe {
            // SAFETY: the input str is guaranteed to have valid UTF-8.
            JavaStr::from_semi_utf8_unchecked(str.as_bytes())
        }
    }

    #[inline]
    #[must_use]
    pub fn from_mut_str(str: &mut str) -> &mut JavaStr {
        unsafe {
            // SAFETY: the input str is guaranteed to have valid UTF-8.
            JavaStr::from_semi_utf8_unchecked_mut(str.as_bytes_mut())
        }
    }

    #[inline]
    #[must_use]
    pub fn from_boxed_str(v: Box<str>) -> Box<JavaStr> {
        unsafe { JavaStr::from_boxed_semi_utf8_unchecked(v.into_boxed_bytes()) }
    }

    /// # Safety
    ///
    /// The parameter must be in semi-valid UTF-8 format, that is, UTF-8 plus
    /// surrogate code points.
    #[inline]
    #[must_use]
    pub unsafe fn from_boxed_semi_utf8_unchecked(v: Box<[u8]>) -> Box<JavaStr> {
        unsafe { Box::from_raw(Box::into_raw(v) as *mut JavaStr) }
    }

    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// # Safety
    ///
    /// The returned slice must not have invalid UTF-8 written to it, besides
    /// surrogate pairs.
    #[inline]
    #[must_use]
    pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    #[inline]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.inner.as_mut_ptr()
    }

    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *const u8 {
        self.inner.as_ptr()
    }

    pub const fn as_str(&self) -> Result<&str, Utf8Error> {
        // Manual implementation of Option::map since it's not const
        match run_utf8_full_validation_from_semi(self.as_bytes()) {
            Ok(..) => unsafe {
                // SAFETY: we were already semi-valid, and full validation just succeeded.
                Ok(self.as_str_unchecked())
            },
            Err(err) => Err(err),
        }
    }

    /// # Safety
    ///
    /// This string must be fully valid UTF-8, i.e. have no surrogate code
    /// points.
    #[inline]
    #[must_use]
    pub const unsafe fn as_str_unchecked(&self) -> &str {
        std::str::from_utf8_unchecked(self.as_bytes())
    }

    #[must_use]
    pub fn as_str_lossy(&self) -> Cow<'_, str> {
        match run_utf8_full_validation_from_semi(self.as_bytes()) {
            Ok(()) => unsafe {
                // SAFETY: validation succeeded
                Cow::Borrowed(self.as_str_unchecked())
            },
            Err(error) => unsafe {
                // SAFETY: invalid parts of string are converted to replacement char
                Cow::Owned(
                    self.transform_invalid_string(error, str::to_owned, |_| {
                        JavaStr::from_str("\u{FFFD}")
                    })
                    .into_string_unchecked(),
                )
            },
        }
    }

    #[inline]
    pub fn bytes(&self) -> Bytes<'_> {
        Bytes {
            inner: self.inner.iter().copied(),
        }
    }

    #[inline]
    pub fn char_indices(&self) -> CharIndices<'_> {
        CharIndices {
            front_offset: 0,
            inner: self.chars(),
        }
    }

    #[inline]
    pub fn chars(&self) -> Chars<'_> {
        Chars {
            inner: self.inner.iter(),
        }
    }

    #[inline]
    #[must_use]
    pub fn contains<P>(&self, mut pat: P) -> bool
    where
        P: JavaStrPattern,
    {
        pat.find_in(self).is_some()
    }

    #[inline]
    #[must_use]
    pub fn ends_with<P>(&self, mut pat: P) -> bool
    where
        P: JavaStrPattern,
    {
        pat.suffix_len_in(self).is_some()
    }

    #[inline]
    #[must_use]
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        self.as_bytes().eq_ignore_ascii_case(other.as_bytes())
    }

    #[inline]
    #[must_use]
    pub fn eq_java_ignore_ascii_case(&self, other: &JavaStr) -> bool {
        self.as_bytes().eq_ignore_ascii_case(other.as_bytes())
    }

    #[inline]
    pub fn escape_debug(&self) -> EscapeDebug<'_> {
        #[inline]
        fn escape_first(first: JavaCodePoint) -> CharEscapeIter {
            first.escape_debug_ext(EscapeDebugExtArgs::ESCAPE_ALL)
        }
        #[inline]
        fn escape_rest(char: JavaCodePoint) -> CharEscapeIter {
            char.escape_debug_ext(EscapeDebugExtArgs {
                escape_single_quote: true,
                escape_double_quote: true,
            })
        }

        let mut chars = self.chars();
        EscapeDebug {
            inner: chars
                .next()
                .map(escape_first as fn(JavaCodePoint) -> CharEscapeIter)
                .into_iter()
                .flatten()
                .chain(chars.flat_map(escape_rest as fn(JavaCodePoint) -> CharEscapeIter)),
        }
    }

    #[inline]
    pub fn escape_default(&self) -> EscapeDefault<'_> {
        EscapeDefault {
            inner: self.chars().flat_map(JavaCodePoint::escape_default),
        }
    }

    #[inline]
    pub fn escape_unicode(&self) -> EscapeUnicode<'_> {
        EscapeUnicode {
            inner: self.chars().flat_map(JavaCodePoint::escape_unicode),
        }
    }

    #[inline]
    #[must_use]
    pub fn find<P>(&self, mut pat: P) -> Option<usize>
    where
        P: JavaStrPattern,
    {
        pat.find_in(self).map(|(index, _)| index)
    }

    #[inline]
    #[must_use]
    pub fn get<I>(&self, i: I) -> Option<&JavaStr>
    where
        I: JavaStrSliceIndex,
    {
        i.get(self)
    }

    #[inline]
    #[must_use]
    pub fn get_mut<I>(&mut self, i: I) -> Option<&mut JavaStr>
    where
        I: JavaStrSliceIndex,
    {
        i.get_mut(self)
    }

    /// # Safety
    ///
    /// - The starting index must not exceed the ending index
    /// - Indexes must be within bounds of the original slice
    /// - Indexes must lie on UTF-8 sequence boundaries
    #[inline]
    #[must_use]
    pub unsafe fn get_unchecked<I>(&self, i: I) -> &JavaStr
    where
        I: JavaStrSliceIndex,
    {
        unsafe { &*i.get_unchecked(self) }
    }

    /// # Safety
    ///
    /// - The starting index must not exceed the ending index
    /// - Indexes must be within bounds of the original slice
    /// - Indexes must lie on UTF-8 sequence boundaries
    #[inline]
    #[must_use]
    pub unsafe fn get_unchecked_mut<I>(&mut self, i: I) -> &mut JavaStr
    where
        I: JavaStrSliceIndex,
    {
        unsafe { &mut *i.get_unchecked_mut(self) }
    }

    #[inline]
    #[must_use]
    pub fn into_boxed_bytes(self: Box<JavaStr>) -> Box<[u8]> {
        unsafe { Box::from_raw(Box::into_raw(self) as *mut [u8]) }
    }

    #[inline]
    #[must_use]
    pub fn into_string(self: Box<JavaStr>) -> JavaString {
        let slice = self.into_boxed_bytes();
        unsafe { JavaString::from_semi_utf8_unchecked(slice.into_vec()) }
    }

    #[inline]
    #[must_use]
    pub fn is_ascii(&self) -> bool {
        self.as_bytes().is_ascii()
    }

    #[inline]
    #[must_use]
    pub fn is_char_boundary(&self, index: usize) -> bool {
        // 0 is always ok.
        // Test for 0 explicitly so that it can optimize out the check
        // easily and skip reading string data for that case.
        // Note that optimizing `self.get(..index)` relies on this.
        if index == 0 {
            return true;
        }

        match self.as_bytes().get(index) {
            // For `None` we have two options:
            //
            // - index == self.len() Empty strings are valid, so return true
            // - index > self.len() In this case return false
            //
            // The check is placed exactly here, because it improves generated
            // code on higher opt-levels. See https://github.com/rust-lang/rust/pull/84751 for more details.
            None => index == self.len(),

            Some(&b) => {
                // This is bit magic equivalent to: b < 128 || b >= 192
                (b as i8) >= -0x40
            }
        }
    }

    pub(crate) fn floor_char_boundary(&self, index: usize) -> usize {
        if index >= self.len() {
            self.len()
        } else {
            let lower_bound = index.saturating_sub(3);
            let new_index = self.as_bytes()[lower_bound..=index].iter().rposition(|b| {
                // This is bit magic equivalent to: b < 128 || b >= 192
                (*b as i8) >= -0x40
            });

            // SAFETY: we know that the character boundary will be within four bytes
            unsafe { lower_bound + new_index.unwrap_unchecked() }
        }
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn lines(&self) -> Lines<'_> {
        Lines {
            inner: self.split_inclusive('\n').map(|line| {
                let Some(line) = line.strip_suffix('\n') else {
                    return line;
                };
                let Some(line) = line.strip_suffix('\r') else {
                    return line;
                };
                line
            }),
        }
    }

    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        // SAFETY: changing ASCII letters only does not invalidate UTF-8.
        let me = unsafe { self.as_bytes_mut() };
        me.make_ascii_lowercase()
    }

    #[inline]
    pub fn make_ascii_uppercase(&mut self) {
        // SAFETY: changing ASCII letters only does not invalidate UTF-8.
        let me = unsafe { self.as_bytes_mut() };
        me.make_ascii_uppercase()
    }

    #[inline]
    pub fn match_indices<P>(&self, pat: P) -> MatchIndices<P>
    where
        P: JavaStrPattern,
    {
        MatchIndices {
            str: self,
            start: 0,
            pat,
        }
    }

    #[inline]
    pub fn matches<P>(&self, pat: P) -> Matches<P>
    where
        P: JavaStrPattern,
    {
        Matches { str: self, pat }
    }

    #[inline]
    pub fn parse<F>(&self) -> Result<F, ParseError<<F as FromStr>::Err>>
    where
        F: FromStr,
    {
        match self.as_str() {
            Ok(str) => str.parse().map_err(ParseError::Err),
            Err(err) => Err(ParseError::InvalidUtf8(err)),
        }
    }

    #[inline]
    #[must_use]
    pub fn repeat(&self, n: usize) -> JavaString {
        unsafe { JavaString::from_semi_utf8_unchecked(self.as_bytes().repeat(n)) }
    }

    #[inline]
    #[must_use]
    pub fn replace<P>(&self, from: P, to: &str) -> JavaString
    where
        P: JavaStrPattern,
    {
        self.replace_java(from, JavaStr::from_str(to))
    }

    #[inline]
    #[must_use]
    pub fn replace_java<P>(&self, from: P, to: &JavaStr) -> JavaString
    where
        P: JavaStrPattern,
    {
        let mut result = JavaString::new();
        let mut last_end = 0;
        for (start, part) in self.match_indices(from) {
            result.push_java_str(unsafe { self.get_unchecked(last_end..start) });
            result.push_java_str(to);
            last_end = start + part.len();
        }
        result.push_java_str(unsafe { self.get_unchecked(last_end..self.len()) });
        result
    }

    #[inline]
    #[must_use]
    pub fn replacen<P>(&self, from: P, to: &str, count: usize) -> JavaString
    where
        P: JavaStrPattern,
    {
        self.replacen_java(from, JavaStr::from_str(to), count)
    }

    #[inline]
    #[must_use]
    pub fn replacen_java<P>(&self, from: P, to: &JavaStr, count: usize) -> JavaString
    where
        P: JavaStrPattern,
    {
        // Hope to reduce the times of re-allocation
        let mut result = JavaString::with_capacity(32);
        let mut last_end = 0;
        for (start, part) in self.match_indices(from).take(count) {
            result.push_java_str(unsafe { self.get_unchecked(last_end..start) });
            result.push_java_str(to);
            last_end = start + part.len();
        }
        result.push_java_str(unsafe { self.get_unchecked(last_end..self.len()) });
        result
    }

    #[inline]
    #[must_use]
    pub fn rfind<P>(&self, mut pat: P) -> Option<usize>
    where
        P: JavaStrPattern,
    {
        pat.rfind_in(self).map(|(index, _)| index)
    }

    #[inline]
    pub fn rmatch_indices<P>(&self, pat: P) -> RMatchIndices<P>
    where
        P: JavaStrPattern,
    {
        RMatchIndices {
            inner: self.match_indices(pat),
        }
    }

    #[inline]
    pub fn rmatches<P>(&self, pat: P) -> RMatches<P>
    where
        P: JavaStrPattern,
    {
        RMatches {
            inner: self.matches(pat),
        }
    }

    #[inline]
    pub fn rsplit<P>(&self, pat: P) -> RSplit<P>
    where
        P: JavaStrPattern,
    {
        RSplit::new(self, pat)
    }

    #[inline]
    #[must_use]
    pub fn rsplit_once<P>(&self, mut delimiter: P) -> Option<(&JavaStr, &JavaStr)>
    where
        P: JavaStrPattern,
    {
        let (index, len) = delimiter.rfind_in(self)?;
        // SAFETY: pattern is known to return valid indices.
        unsafe {
            Some((
                self.get_unchecked(..index),
                self.get_unchecked(index + len..),
            ))
        }
    }

    #[inline]
    pub fn rsplit_terminator<P>(&self, pat: P) -> RSplitTerminator<P>
    where
        P: JavaStrPattern,
    {
        RSplitTerminator::new(self, pat)
    }

    #[inline]
    pub fn rsplitn<P>(&self, n: usize, pat: P) -> RSplitN<P>
    where
        P: JavaStrPattern,
    {
        RSplitN::new(self, pat, n)
    }

    #[inline]
    pub fn split<P>(&self, pat: P) -> Split<P>
    where
        P: JavaStrPattern,
    {
        Split::new(self, pat)
    }

    #[inline]
    pub fn split_ascii_whitespace(&self) -> SplitAsciiWhitespace<'_> {
        #[inline]
        fn is_non_empty(bytes: &&[u8]) -> bool {
            !bytes.is_empty()
        }

        SplitAsciiWhitespace {
            inner: self
                .as_bytes()
                .split(u8::is_ascii_whitespace as fn(&u8) -> bool)
                .filter(is_non_empty as fn(&&[u8]) -> bool)
                .map(|bytes| unsafe { JavaStr::from_semi_utf8_unchecked(bytes) }),
        }
    }

    #[inline]
    #[must_use]
    pub fn split_at(&self, mid: usize) -> (&JavaStr, &JavaStr) {
        // is_char_boundary checks that the index is in [0, .len()]
        if self.is_char_boundary(mid) {
            // SAFETY: just checked that `mid` is on a char boundary.
            unsafe {
                (
                    self.get_unchecked(0..mid),
                    self.get_unchecked(mid..self.len()),
                )
            }
        } else {
            slice_error_fail(self, 0, mid)
        }
    }

    #[inline]
    #[must_use]
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut JavaStr, &mut JavaStr) {
        // is_char_boundary checks that the index is in [0, .len()]
        if self.is_char_boundary(mid) {
            let len = self.len();
            let ptr = self.as_mut_ptr();
            // SAFETY: just checked that `mid` is on a char boundary.
            unsafe {
                (
                    JavaStr::from_semi_utf8_unchecked_mut(slice::from_raw_parts_mut(ptr, mid)),
                    JavaStr::from_semi_utf8_unchecked_mut(slice::from_raw_parts_mut(
                        ptr.add(mid),
                        len - mid,
                    )),
                )
            }
        } else {
            slice_error_fail(self, 0, mid)
        }
    }

    #[inline]
    pub fn split_inclusive<P>(&self, pat: P) -> SplitInclusive<P>
    where
        P: JavaStrPattern,
    {
        SplitInclusive::new(self, pat)
    }

    #[inline]
    #[must_use]
    pub fn split_once<P>(&self, mut delimiter: P) -> Option<(&JavaStr, &JavaStr)>
    where
        P: JavaStrPattern,
    {
        let (index, len) = delimiter.find_in(self)?;
        // SAFETY: pattern is known to return valid indices.
        unsafe {
            Some((
                self.get_unchecked(..index),
                self.get_unchecked(index + len..),
            ))
        }
    }

    #[inline]
    pub fn split_terminator<P>(&self, pat: P) -> SplitTerminator<P>
    where
        P: JavaStrPattern,
    {
        SplitTerminator::new(self, pat)
    }

    #[inline]
    pub fn split_whitespace(&self) -> SplitWhitespace<'_> {
        SplitWhitespace {
            inner: self
                .split(JavaCodePoint::is_whitespace as fn(JavaCodePoint) -> bool)
                .filter(|str| !str.is_empty()),
        }
    }

    #[inline]
    pub fn splitn<P>(&self, n: usize, pat: P) -> SplitN<P>
    where
        P: JavaStrPattern,
    {
        SplitN::new(self, pat, n)
    }

    #[inline]
    #[must_use]
    pub fn starts_with<P>(&self, mut pat: P) -> bool
    where
        P: JavaStrPattern,
    {
        pat.prefix_len_in(self).is_some()
    }

    #[inline]
    #[must_use]
    pub fn strip_prefix<P>(&self, mut prefix: P) -> Option<&JavaStr>
    where
        P: JavaStrPattern,
    {
        let len = prefix.prefix_len_in(self)?;
        // SAFETY: pattern is known to return valid indices.
        unsafe { Some(self.get_unchecked(len..)) }
    }

    #[inline]
    #[must_use]
    pub fn strip_suffix<P>(&self, mut suffix: P) -> Option<&JavaStr>
    where
        P: JavaStrPattern,
    {
        let len = suffix.suffix_len_in(self)?;
        // SAFETY: pattern is known to return valid indices.
        unsafe { Some(self.get_unchecked(..self.len() - len)) }
    }

    #[inline]
    #[must_use]
    pub fn to_ascii_lowercase(&self) -> JavaString {
        let mut s = self.to_owned();
        s.make_ascii_lowercase();
        s
    }

    #[inline]
    #[must_use]
    pub fn to_ascii_uppercase(&self) -> JavaString {
        let mut s = self.to_owned();
        s.make_ascii_uppercase();
        s
    }

    #[inline]
    #[must_use]
    pub fn to_lowercase(&self) -> JavaString {
        self.transform_string(str::to_lowercase, |ch| ch)
    }

    #[inline]
    #[must_use]
    pub fn to_uppercase(&self) -> JavaString {
        self.transform_string(str::to_uppercase, |ch| ch)
    }

    #[inline]
    #[must_use]
    pub fn trim(&self) -> &JavaStr {
        self.trim_matches(|c: JavaCodePoint| c.is_whitespace())
    }

    #[inline]
    #[must_use]
    pub fn trim_end(&self) -> &JavaStr {
        self.trim_end_matches(|c: JavaCodePoint| c.is_whitespace())
    }

    #[inline]
    #[must_use]
    pub fn trim_end_matches<P>(&self, mut pat: P) -> &JavaStr
    where
        P: JavaStrPattern,
    {
        let mut str = self;
        while let Some(suffix_len) = pat.suffix_len_in(str) {
            if suffix_len == 0 {
                break;
            }
            // SAFETY: pattern is known to return valid indices.
            str = unsafe { str.get_unchecked(..str.len() - suffix_len) };
        }
        str
    }

    #[inline]
    #[must_use]
    pub fn trim_matches<P>(&self, mut pat: P) -> &JavaStr
    where
        P: JavaStrPattern,
    {
        let mut str = self;
        while let Some(prefix_len) = pat.prefix_len_in(str) {
            if prefix_len == 0 {
                break;
            }
            // SAFETY: pattern is known to return valid indices.
            str = unsafe { str.get_unchecked(prefix_len..) };
        }
        while let Some(suffix_len) = pat.suffix_len_in(str) {
            if suffix_len == 0 {
                break;
            }
            // SAFETY: pattern is known to return valid indices.
            str = unsafe { str.get_unchecked(..str.len() - suffix_len) };
        }
        str
    }

    #[inline]
    #[must_use]
    pub fn trim_start(&self) -> &JavaStr {
        self.trim_start_matches(|c: JavaCodePoint| c.is_whitespace())
    }

    #[inline]
    #[must_use]
    pub fn trim_start_matches<P>(&self, mut pat: P) -> &JavaStr
    where
        P: JavaStrPattern,
    {
        let mut str = self;
        while let Some(prefix_len) = pat.prefix_len_in(str) {
            if prefix_len == 0 {
                break;
            }
            // SAFETY: pattern is known to return valid indices.
            str = unsafe { str.get_unchecked(prefix_len..) };
        }
        str
    }

    #[inline]
    fn transform_string<SF, ICF>(
        &self,
        mut string_transformer: SF,
        invalid_char_transformer: ICF,
    ) -> JavaString
    where
        SF: FnMut(&str) -> String,
        ICF: FnMut(&JavaStr) -> &JavaStr,
    {
        let bytes = self.as_bytes();
        match run_utf8_full_validation_from_semi(bytes) {
            Ok(()) => JavaString::from(string_transformer(unsafe {
                // SAFETY: validation succeeded
                std::str::from_utf8_unchecked(bytes)
            })),
            Err(error) => {
                self.transform_invalid_string(error, string_transformer, invalid_char_transformer)
            }
        }
    }

    #[inline]
    fn transform_invalid_string<SF, ICF>(
        &self,
        error: Utf8Error,
        mut string_transformer: SF,
        mut invalid_char_transformer: ICF,
    ) -> JavaString
    where
        SF: FnMut(&str) -> String,
        ICF: FnMut(&JavaStr) -> &JavaStr,
    {
        let bytes = self.as_bytes();
        let mut result = JavaString::from(string_transformer(unsafe {
            // SAFETY: validation succeeded up to this index
            std::str::from_utf8_unchecked(bytes.get_unchecked(..error.valid_up_to))
        }));
        result.push_java_str(invalid_char_transformer(unsafe {
            // SAFETY: any UTF-8 error in semi-valid UTF-8 is a 3 byte long sequence
            // representing a surrogate code point. We're pushing that sequence now
            JavaStr::from_semi_utf8_unchecked(
                bytes.get_unchecked(error.valid_up_to..error.valid_up_to + 3),
            )
        }));
        let mut index = error.valid_up_to + 3;
        loop {
            let remainder = unsafe { bytes.get_unchecked(index..) };
            match run_utf8_full_validation_from_semi(remainder) {
                Ok(()) => {
                    result.push_str(&string_transformer(unsafe {
                        // SAFETY: validation succeeded
                        std::str::from_utf8_unchecked(remainder)
                    }));
                    return result;
                }
                Err(error) => {
                    result.push_str(&string_transformer(unsafe {
                        // SAFETY: validation succeeded up to this index
                        std::str::from_utf8_unchecked(
                            bytes.get_unchecked(index..index + error.valid_up_to),
                        )
                    }));
                    result.push_java_str(invalid_char_transformer(unsafe {
                        // SAFETY: see comment above
                        JavaStr::from_semi_utf8_unchecked(bytes.get_unchecked(
                            index + error.valid_up_to..index + error.valid_up_to + 3,
                        ))
                    }));
                    index += error.valid_up_to + 3;
                }
            }
        }
    }
}

impl<'a> Add<&JavaStr> for Cow<'a, JavaStr> {
    type Output = Cow<'a, JavaStr>;

    #[inline]
    fn add(mut self, rhs: &JavaStr) -> Self::Output {
        self += rhs;
        self
    }
}

impl<'a> AddAssign<&JavaStr> for Cow<'a, JavaStr> {
    #[inline]
    fn add_assign(&mut self, rhs: &JavaStr) {
        if !rhs.is_empty() {
            match self {
                Cow::Borrowed(lhs) => {
                    let mut result = lhs.to_owned();
                    result.push_java_str(rhs);
                    *self = Cow::Owned(result);
                }
                Cow::Owned(lhs) => {
                    lhs.push_java_str(rhs);
                }
            }
        }
    }
}

impl AsRef<[u8]> for JavaStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Clone for Box<JavaStr> {
    #[inline]
    fn clone(&self) -> Self {
        let buf: Box<[u8]> = self.as_bytes().into();
        unsafe { JavaStr::from_boxed_semi_utf8_unchecked(buf) }
    }
}

impl Debug for JavaStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('"')?;
        let mut from = 0;
        for (i, c) in self.char_indices() {
            let esc = c.escape_debug_ext(EscapeDebugExtArgs {
                escape_single_quote: false,
                escape_double_quote: true,
            });
            // If char needs escaping, flush backlog so far and write, else skip.
            // Also handle invalid UTF-8 here
            if esc.len() != 1 || c.as_char().is_none() {
                unsafe {
                    // SAFETY: any invalid UTF-8 should have been caught by a previous iteration
                    f.write_str(self[from..i].as_str_unchecked())?;
                }
                for c in esc {
                    f.write_char(c)?;
                }
                from = i + c.len_utf8();
            }
        }
        unsafe {
            // SAFETY: any invalid UTF-8 should have been caught by the loop above
            f.write_str(self[from..].as_str_unchecked())?;
        }
        f.write_char('"')
    }
}

impl Default for &JavaStr {
    #[inline]
    fn default() -> Self {
        JavaStr::from_str("")
    }
}

impl Default for Box<JavaStr> {
    #[inline]
    fn default() -> Self {
        JavaStr::from_boxed_str(Box::<str>::default())
    }
}

impl Display for JavaStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.as_str_lossy(), f)
    }
}

impl<'a> From<&'a JavaStr> for Cow<'a, JavaStr> {
    #[inline]
    fn from(value: &'a JavaStr) -> Self {
        Cow::Borrowed(value)
    }
}

impl From<&JavaStr> for Arc<JavaStr> {
    #[inline]
    fn from(value: &JavaStr) -> Self {
        let arc = Arc::<[u8]>::from(value.as_bytes());
        unsafe { Arc::from_raw(Arc::into_raw(arc) as *const JavaStr) }
    }
}

impl From<&JavaStr> for Box<JavaStr> {
    #[inline]
    fn from(value: &JavaStr) -> Self {
        unsafe { JavaStr::from_boxed_semi_utf8_unchecked(Box::from(value.as_bytes())) }
    }
}

impl From<&JavaStr> for Rc<JavaStr> {
    #[inline]
    fn from(value: &JavaStr) -> Self {
        let rc = Rc::<[u8]>::from(value.as_bytes());
        unsafe { Rc::from_raw(Rc::into_raw(rc) as *const JavaStr) }
    }
}

impl From<&JavaStr> for Vec<u8> {
    #[inline]
    fn from(value: &JavaStr) -> Self {
        From::from(value.as_bytes())
    }
}

impl From<Cow<'_, JavaStr>> for Box<JavaStr> {
    #[inline]
    fn from(value: Cow<'_, JavaStr>) -> Self {
        match value {
            Cow::Borrowed(s) => Box::from(s),
            Cow::Owned(s) => Box::from(s),
        }
    }
}

impl From<JavaString> for Box<JavaStr> {
    #[inline]
    fn from(value: JavaString) -> Self {
        value.into_boxed_str()
    }
}

impl<'a> From<&'a str> for &'a JavaStr {
    #[inline]
    fn from(value: &'a str) -> Self {
        JavaStr::from_str(value)
    }
}

impl Hash for JavaStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.as_bytes());
        state.write_u8(0xff);
    }
}

impl<I> Index<I> for JavaStr
where
    I: JavaStrSliceIndex,
{
    type Output = JavaStr;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        index.index(self)
    }
}

impl<I> IndexMut<I> for JavaStr
where
    I: JavaStrSliceIndex,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.index_mut(self)
    }
}

impl<'a, 'b> PartialEq<&'b JavaStr> for Cow<'a, str> {
    #[inline]
    fn eq(&self, other: &&'b JavaStr) -> bool {
        self == *other
    }
}

impl<'a, 'b> PartialEq<&'b JavaStr> for Cow<'a, JavaStr> {
    #[inline]
    fn eq(&self, other: &&'b JavaStr) -> bool {
        self == *other
    }
}

impl<'a, 'b> PartialEq<Cow<'a, str>> for &'b JavaStr {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        *self == other
    }
}

impl<'a> PartialEq<Cow<'a, str>> for JavaStr {
    #[inline]
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        other == self
    }
}

impl<'a, 'b> PartialEq<Cow<'a, JavaStr>> for &'b JavaStr {
    #[inline]
    fn eq(&self, other: &Cow<'a, JavaStr>) -> bool {
        *self == other
    }
}

impl<'a> PartialEq<Cow<'a, JavaStr>> for JavaStr {
    #[inline]
    fn eq(&self, other: &Cow<'a, JavaStr>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<String> for &'a JavaStr {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        *self == other
    }
}

impl PartialEq<String> for JavaStr {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self == &other[..]
    }
}

impl PartialEq<JavaStr> for String {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        &self[..] == other
    }
}

impl<'a> PartialEq<JavaString> for &'a JavaStr {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        *self == other
    }
}

impl PartialEq<JavaString> for JavaStr {
    #[inline]
    fn eq(&self, other: &JavaString) -> bool {
        self == other[..]
    }
}

impl<'a> PartialEq<JavaStr> for Cow<'a, str> {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        match self {
            Cow::Borrowed(this) => this == other,
            Cow::Owned(this) => this == other,
        }
    }
}

impl<'a> PartialEq<JavaStr> for Cow<'a, JavaStr> {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        match self {
            Cow::Borrowed(this) => this == other,
            Cow::Owned(this) => this == other,
        }
    }
}

impl PartialEq<JavaStr> for str {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        JavaStr::from_str(self) == other
    }
}

impl<'a> PartialEq<JavaStr> for &'a str {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        *self == other
    }
}

impl PartialEq<str> for JavaStr {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self == JavaStr::from_str(other)
    }
}

impl<'a> PartialEq<&'a str> for JavaStr {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<JavaStr> for &'a JavaStr {
    #[inline]
    fn eq(&self, other: &JavaStr) -> bool {
        *self == other
    }
}

impl<'a> PartialEq<&'a JavaStr> for JavaStr {
    #[inline]
    fn eq(&self, other: &&'a JavaStr) -> bool {
        self == *other
    }
}

impl ToOwned for JavaStr {
    type Owned = JavaString;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        unsafe { JavaString::from_semi_utf8_unchecked(self.as_bytes().to_vec()) }
    }
}

mod private_slice_index {
    use std::ops;

    pub trait Sealed {}

    impl Sealed for ops::Range<usize> {}
    impl Sealed for ops::RangeTo<usize> {}
    impl Sealed for ops::RangeFrom<usize> {}
    impl Sealed for ops::RangeFull {}
    impl Sealed for ops::RangeInclusive<usize> {}
    impl Sealed for ops::RangeToInclusive<usize> {}
}

/// # Safety
///
/// Implementations' `check_bounds` method must properly check the bounds of the
/// slice, such that calling `get_unchecked` is not UB.
pub unsafe trait JavaStrSliceIndex: private_slice_index::Sealed + Sized {
    fn check_bounds(&self, slice: &JavaStr) -> bool;
    fn check_bounds_fail(self, slice: &JavaStr) -> !;

    /// # Safety
    ///
    /// - The input slice must be a valid pointer
    /// - This index must not be out of bounds of the input slice
    /// - The indices of this slice must point to char boundaries in the input
    ///   slice
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr;

    /// # Safety
    ///
    /// - The input slice must be a valid pointer
    /// - This index must not be out of bounds of the input slice
    /// - The indices of this slice must point to char boundaries in the input
    ///   slice
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr;

    #[inline]
    fn get(self, slice: &JavaStr) -> Option<&JavaStr> {
        if self.check_bounds(slice) {
            Some(unsafe { &*self.get_unchecked(slice) })
        } else {
            None
        }
    }

    #[inline]
    fn get_mut(self, slice: &mut JavaStr) -> Option<&mut JavaStr> {
        if self.check_bounds(slice) {
            Some(unsafe { &mut *self.get_unchecked_mut(slice) })
        } else {
            None
        }
    }

    #[inline]
    fn index(self, slice: &JavaStr) -> &JavaStr {
        if self.check_bounds(slice) {
            unsafe { &*self.get_unchecked(slice) }
        } else {
            self.check_bounds_fail(slice)
        }
    }

    #[inline]
    fn index_mut(self, slice: &mut JavaStr) -> &mut JavaStr {
        if self.check_bounds(slice) {
            unsafe { &mut *self.get_unchecked_mut(slice) }
        } else {
            self.check_bounds_fail(slice)
        }
    }
}

unsafe impl JavaStrSliceIndex for RangeFull {
    #[inline]
    fn check_bounds(&self, _slice: &JavaStr) -> bool {
        true
    }

    #[inline]
    fn check_bounds_fail(self, _slice: &JavaStr) -> ! {
        unreachable!()
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr {
        slice
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr {
        slice
    }
}

unsafe impl JavaStrSliceIndex for Range<usize> {
    #[inline]
    fn check_bounds(&self, slice: &JavaStr) -> bool {
        self.start <= self.end
            && slice.is_char_boundary(self.start)
            && slice.is_char_boundary(self.end)
    }

    #[inline]
    #[track_caller]
    fn check_bounds_fail(self, slice: &JavaStr) -> ! {
        slice_error_fail(slice, self.start, self.end)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr {
        let slice = slice as *const [u8];
        // SAFETY: the caller guarantees that `self` is in bounds of `slice`
        // which satisfies all the conditions for `add`.
        let ptr = unsafe { (slice as *const u8).add(self.start) };
        let len = self.end - self.start;
        ptr::slice_from_raw_parts(ptr, len) as *const JavaStr
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr {
        let slice = slice as *mut [u8];
        // SAFETY: see comments for `get_unchecked`.
        let ptr = unsafe { (slice as *mut u8).add(self.start) };
        let len = self.end - self.start;
        ptr::slice_from_raw_parts_mut(ptr, len) as *mut JavaStr
    }
}

unsafe impl JavaStrSliceIndex for RangeTo<usize> {
    #[inline]
    fn check_bounds(&self, slice: &JavaStr) -> bool {
        slice.is_char_boundary(self.end)
    }

    #[inline]
    #[track_caller]
    fn check_bounds_fail(self, slice: &JavaStr) -> ! {
        slice_error_fail(slice, 0, self.end)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr {
        unsafe { (0..self.end).get_unchecked(slice) }
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr {
        unsafe { (0..self.end).get_unchecked_mut(slice) }
    }
}

unsafe impl JavaStrSliceIndex for RangeFrom<usize> {
    #[inline]
    fn check_bounds(&self, slice: &JavaStr) -> bool {
        slice.is_char_boundary(self.start)
    }

    #[inline]
    #[track_caller]
    fn check_bounds_fail(self, slice: &JavaStr) -> ! {
        slice_error_fail(slice, self.start, slice.len())
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr {
        let len = unsafe { (*(slice as *const [u8])).len() };
        unsafe { (self.start..len).get_unchecked(slice) }
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr {
        let len = unsafe { (*(slice as *mut [u8])).len() };
        unsafe { (self.start..len).get_unchecked_mut(slice) }
    }
}

#[inline]
fn into_slice_range(range: RangeInclusive<usize>) -> Range<usize> {
    let exclusive_end = *range.end() + 1;
    let start = match range.end_bound() {
        Bound::Excluded(..) => exclusive_end, // excluded
        Bound::Included(..) => *range.start(),
        Bound::Unbounded => unreachable!(),
    };
    start..exclusive_end
}

unsafe impl JavaStrSliceIndex for RangeInclusive<usize> {
    #[inline]
    fn check_bounds(&self, slice: &JavaStr) -> bool {
        *self.end() != usize::MAX && into_slice_range(self.clone()).check_bounds(slice)
    }

    #[inline]
    #[track_caller]
    fn check_bounds_fail(self, slice: &JavaStr) -> ! {
        if *self.end() == usize::MAX {
            str_end_index_overflow_fail()
        } else {
            into_slice_range(self).check_bounds_fail(slice)
        }
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr {
        into_slice_range(self).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr {
        into_slice_range(self).get_unchecked_mut(slice)
    }
}

unsafe impl JavaStrSliceIndex for RangeToInclusive<usize> {
    #[inline]
    fn check_bounds(&self, slice: &JavaStr) -> bool {
        (0..=self.end).check_bounds(slice)
    }

    #[inline]
    fn check_bounds_fail(self, slice: &JavaStr) -> ! {
        (0..=self.end).check_bounds_fail(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const JavaStr) -> *const JavaStr {
        (0..=self.end).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut JavaStr) -> *mut JavaStr {
        (0..=self.end).get_unchecked_mut(slice)
    }
}
