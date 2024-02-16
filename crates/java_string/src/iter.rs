use std::fmt::{Debug, Display, Formatter, Write};
use std::iter::{Chain, Copied, Filter, FlatMap, Flatten, FusedIterator, Map};
use std::{option, slice};

use crate::validations::{next_code_point, next_code_point_reverse};
use crate::{CharEscapeIter, JavaCodePoint, JavaStr, JavaStrPattern};
macro_rules! delegate {
    (Iterator for $ty:ident $(<$($lt:lifetime),+>)? => $item:ty $(, DoubleEnded = $double_ended:ty)?) => {
        impl$(<$($lt),+>)? Iterator for $ty$(<$($lt),+>)? {
            type Item = $item;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }

            #[inline]
            fn count(self) -> usize {
                self.inner.count()
            }

            #[inline]
            fn last(self) -> Option<Self::Item> {
                self.inner.last()
            }

            #[inline]
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.inner.nth(n)
            }

            #[inline]
            fn all<F>(&mut self, f: F) -> bool
            where
                F: FnMut(Self::Item) -> bool,
            {
                self.inner.all(f)
            }

            #[inline]
            fn any<F>(&mut self, f: F) -> bool
            where
                F: FnMut(Self::Item) -> bool,
            {
                self.inner.any(f)
            }

            #[inline]
            fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
            where
                P: FnMut(&Self::Item) -> bool,
            {
                self.inner.find(predicate)
            }

            #[inline]
            fn position<P>(&mut self, predicate: P) -> Option<usize>
            where
                P: FnMut(Self::Item) -> bool,
            {
                self.inner.position(predicate)
            }

            $(
            #[inline]
            fn rposition<P>(&mut self, predicate: P) -> Option<usize>
            where
                P: FnMut(Self::Item) -> bool,
            {
                let _test: $double_ended = ();
                self.inner.rposition(predicate)
            }
            )?
        }
    };

    (DoubleEndedIterator for $ty:ident $(<$($lt:lifetime),+>)?) => {
        impl$(<$($lt),+>)? DoubleEndedIterator for $ty$(<$($lt),+>)? {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.inner.next_back()
            }

            #[inline]
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.inner.nth_back(n)
            }

            #[inline]
            fn rfind<P>(&mut self, predicate: P) -> Option<Self::Item>
            where
                P: FnMut(&Self::Item) -> bool,
            {
                self.inner.rfind(predicate)
            }
        }
    };

    (ExactSizeIterator for $ty:ident $(<$($lt:lifetime),+>)?) => {
        impl$(<$($lt),+>)? ExactSizeIterator for $ty$(<$($lt),+>)? {
            #[inline]
            fn len(&self) -> usize {
                self.inner.len()
            }
        }
    };

    (FusedIterator for $ty:ident $(<$($lt:lifetime),+>)?) => {
        impl$(<$($lt),+>)? FusedIterator for $ty$(<$($lt),+>)? {}
    };

    (Iterator, DoubleEndedIterator, ExactSizeIterator, FusedIterator for $ty:ident $(<$($lt:lifetime),+>)? => $item:ty) => {
        delegate!(Iterator for $ty$(<$($lt),+>)? => $item, DoubleEnded = ());
        delegate!(DoubleEndedIterator for $ty$(<$($lt),+>)?);
        delegate!(ExactSizeIterator for $ty$(<$($lt),+>)?);
        delegate!(FusedIterator for $ty$(<$($lt),+>)?);
    };
}

#[must_use]
#[derive(Clone, Debug)]
pub struct Bytes<'a> {
    pub(crate) inner: Copied<slice::Iter<'a, u8>>,
}
delegate!(Iterator, DoubleEndedIterator, ExactSizeIterator, FusedIterator for Bytes<'a> => u8);

#[derive(Clone, Debug)]
#[must_use]
pub struct EscapeDebug<'a> {
    #[allow(clippy::type_complexity)]
    pub(crate) inner: Chain<
        Flatten<option::IntoIter<CharEscapeIter>>,
        FlatMap<Chars<'a>, CharEscapeIter, fn(JavaCodePoint) -> CharEscapeIter>,
    >,
}
delegate!(Iterator for EscapeDebug<'a> => char);
delegate!(FusedIterator for EscapeDebug<'a>);
impl<'a> Display for EscapeDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct EscapeDefault<'a> {
    pub(crate) inner: FlatMap<Chars<'a>, CharEscapeIter, fn(JavaCodePoint) -> CharEscapeIter>,
}
delegate!(Iterator for EscapeDefault<'a> => char);
delegate!(FusedIterator for EscapeDefault<'a>);
impl<'a> Display for EscapeDefault<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct EscapeUnicode<'a> {
    pub(crate) inner: FlatMap<Chars<'a>, CharEscapeIter, fn(JavaCodePoint) -> CharEscapeIter>,
}
delegate!(Iterator for EscapeUnicode<'a> => char);
delegate!(FusedIterator for EscapeUnicode<'a>);
impl<'a> Display for EscapeUnicode<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct Lines<'a> {
    pub(crate) inner: Map<SplitInclusive<'a, char>, fn(&JavaStr) -> &JavaStr>,
}
delegate!(Iterator for Lines<'a> => &'a JavaStr);
delegate!(DoubleEndedIterator for Lines<'a>);
delegate!(FusedIterator for Lines<'a>);

#[derive(Clone)]
#[must_use]
pub struct Chars<'a> {
    pub(crate) inner: slice::Iter<'a, u8>,
}

impl<'a> Iterator for Chars<'a> {
    type Item = JavaCodePoint;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: `JavaStr` invariant says `self.inner` is a semi-valid UTF-8 string
        // and the resulting `ch` is a valid Unicode Scalar Value or surrogate
        // code point.
        unsafe { next_code_point(&mut self.inner).map(|ch| JavaCodePoint::from_u32_unchecked(ch)) }
    }

    // TODO: std has an optimized count impl

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len();
        // `(len + 3)` can't overflow, because we know that the `slice::Iter`
        // belongs to a slice in memory which has a maximum length of
        // `isize::MAX` (that's well below `usize::MAX`).
        ((len + 3) / 4, Some(len))
    }

    #[inline]
    fn last(mut self) -> Option<JavaCodePoint> {
        // No need to go through the entire string.
        self.next_back()
    }
}

impl Debug for Chars<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chars(")?;
        f.debug_list().entries(self.clone()).finish()?;
        write!(f, ")")?;
        Ok(())
    }
}

impl<'a> DoubleEndedIterator for Chars<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY: `JavaStr` invariant says `self.inner` is a semi-valid UTF-8 string
        // and the resulting `ch` is a valid Unicode Scalar Value or surrogate
        // code point.
        unsafe {
            next_code_point_reverse(&mut self.inner).map(|ch| JavaCodePoint::from_u32_unchecked(ch))
        }
    }
}

impl FusedIterator for Chars<'_> {}

impl<'a> Chars<'a> {
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &'a JavaStr {
        // SAFETY: `Chars` is only made from a JavaStr, which guarantees the iter is
        // semi-valid UTF-8.
        unsafe { JavaStr::from_semi_utf8_unchecked(self.inner.as_slice()) }
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct CharIndices<'a> {
    pub(crate) front_offset: usize,
    pub(crate) inner: Chars<'a>,
}

impl<'a> Iterator for CharIndices<'a> {
    type Item = (usize, JavaCodePoint);

    #[inline]
    fn next(&mut self) -> Option<(usize, JavaCodePoint)> {
        let pre_len = self.inner.inner.len();
        match self.inner.next() {
            None => None,
            Some(ch) => {
                let index = self.front_offset;
                let len = self.inner.inner.len();
                self.front_offset += pre_len - len;
                Some((index, ch))
            }
        }
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn last(mut self) -> Option<(usize, JavaCodePoint)> {
        // No need to go through the entire string.
        self.next_back()
    }
}

impl<'a> DoubleEndedIterator for CharIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, JavaCodePoint)> {
        self.inner.next_back().map(|ch| {
            let index = self.front_offset + self.inner.inner.len();
            (index, ch)
        })
    }
}

impl FusedIterator for CharIndices<'_> {}

impl<'a> CharIndices<'a> {
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &'a JavaStr {
        self.inner.as_str()
    }
}

#[must_use]
#[derive(Debug, Clone)]
pub struct Matches<'a, P> {
    pub(crate) str: &'a JavaStr,
    pub(crate) pat: P,
}

impl<'a, P> Iterator for Matches<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, len)) = self.pat.find_in(self.str) {
            // SAFETY: pattern returns valid indices
            let ret = unsafe { self.str.get_unchecked(index..index + len) };
            self.str = unsafe { self.str.get_unchecked(index + len..) };
            Some(ret)
        } else {
            self.str = Default::default();
            None
        }
    }
}

impl<'a, P> DoubleEndedIterator for Matches<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some((index, len)) = self.pat.rfind_in(self.str) {
            // SAFETY: pattern returns valid indices
            let ret = unsafe { self.str.get_unchecked(index..index + len) };
            self.str = unsafe { self.str.get_unchecked(..index) };
            Some(ret)
        } else {
            self.str = Default::default();
            None
        }
    }
}

#[must_use]
#[derive(Clone, Debug)]
pub struct RMatches<'a, P> {
    pub(crate) inner: Matches<'a, P>,
}

impl<'a, P> Iterator for RMatches<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, P> DoubleEndedIterator for RMatches<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[must_use]
#[derive(Clone, Debug)]
pub struct MatchIndices<'a, P> {
    pub(crate) str: &'a JavaStr,
    pub(crate) start: usize,
    pub(crate) pat: P,
}

impl<'a, P> Iterator for MatchIndices<'a, P>
where
    P: JavaStrPattern,
{
    type Item = (usize, &'a JavaStr);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, len)) = self.pat.find_in(self.str) {
            let full_index = self.start + index;
            self.start = full_index + len;
            // SAFETY: pattern returns valid indices
            let ret = unsafe { self.str.get_unchecked(index..index + len) };
            self.str = unsafe { self.str.get_unchecked(index + len..) };
            Some((full_index, ret))
        } else {
            self.start += self.str.len();
            self.str = Default::default();
            None
        }
    }
}

impl<'a, P> DoubleEndedIterator for MatchIndices<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some((index, len)) = self.pat.rfind_in(self.str) {
            // SAFETY: pattern returns valid indices
            let ret = unsafe { self.str.get_unchecked(index..index + len) };
            self.str = unsafe { self.str.get_unchecked(..index) };
            Some((self.start + index, ret))
        } else {
            self.str = Default::default();
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct RMatchIndices<'a, P> {
    pub(crate) inner: MatchIndices<'a, P>,
}

impl<'a, P> Iterator for RMatchIndices<'a, P>
where
    P: JavaStrPattern,
{
    type Item = (usize, &'a JavaStr);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, P> DoubleEndedIterator for RMatchIndices<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[derive(Clone, Debug)]
struct SplitHelper<'a, P> {
    start: usize,
    end: usize,
    haystack: &'a JavaStr,
    pat: P,
    allow_trailing_empty: bool,
    finished: bool,
    had_empty_match: bool,
}

impl<'a, P> SplitHelper<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn new(haystack: &'a JavaStr, pat: P, allow_trailing_empty: bool) -> Self {
        Self {
            start: 0,
            end: haystack.len(),
            haystack,
            pat,
            allow_trailing_empty,
            finished: false,
            had_empty_match: false,
        }
    }

    #[inline]
    fn get_end(&mut self) -> Option<&'a JavaStr> {
        if !self.finished {
            self.finished = true;

            if self.allow_trailing_empty || self.end - self.start > 0 {
                // SAFETY: `self.start` and `self.end` always lie on unicode boundaries.
                let string = unsafe { self.haystack.get_unchecked(self.start..self.end) };
                return Some(string);
            }
        }

        None
    }

    #[inline]
    fn next_match(&mut self) -> Option<(usize, usize)> {
        // SAFETY: `self.start` always lies on a unicode boundary.
        let substr = unsafe { self.haystack.get_unchecked(self.start..) };

        let result = if self.had_empty_match {
            // if we had an empty match before, we are going to find the empty match again.
            // don't do that, search from the next index along.

            if substr.is_empty() {
                None
            } else {
                // SAFETY: we can pop the string because we already checked if the string is
                // empty above
                let first_char_len = unsafe { substr.chars().next().unwrap_unchecked().len_utf8() };
                let popped_str = unsafe { substr.get_unchecked(first_char_len..) };

                self.pat
                    .find_in(popped_str)
                    .map(|(index, len)| (index + first_char_len + self.start, len))
            }
        } else {
            self.pat
                .find_in(substr)
                .map(|(index, len)| (index + self.start, len))
        };

        self.had_empty_match = result.is_some_and(|(_, len)| len == 0);

        result
    }

    #[inline]
    fn next(&mut self) -> Option<&'a JavaStr> {
        if self.finished {
            return None;
        }

        match self.next_match() {
            Some((index, len)) => unsafe {
                // SAFETY: pattern guarantees valid indices
                let elt = self.haystack.get_unchecked(self.start..index);
                self.start = index + len;
                Some(elt)
            },
            None => self.get_end(),
        }
    }

    #[inline]
    fn next_inclusive(&mut self) -> Option<&'a JavaStr> {
        if self.finished {
            return None;
        }

        match self.next_match() {
            Some((index, len)) => unsafe {
                // SAFETY: pattern guarantees valid indices
                let elt = self.haystack.get_unchecked(self.start..index + len);
                self.start = index + len;
                Some(elt)
            },
            None => self.get_end(),
        }
    }

    #[inline]
    fn next_match_back(&mut self) -> Option<(usize, usize)> {
        // SAFETY: `self.end` always lies on a unicode boundary.
        let substr = unsafe { self.haystack.get_unchecked(..self.end) };

        let result = if self.had_empty_match {
            // if we had an empty match before, we are going to find the empty match again.
            // don't do that, search from the next index along.

            if substr.is_empty() {
                None
            } else {
                // SAFETY: we can pop the string because we already checked if the string is
                // empty above
                let last_char_len =
                    unsafe { substr.chars().next_back().unwrap_unchecked().len_utf8() };
                let popped_str = unsafe { substr.get_unchecked(..substr.len() - last_char_len) };

                self.pat.rfind_in(popped_str)
            }
        } else {
            self.pat.rfind_in(substr)
        };

        self.had_empty_match = result.is_some_and(|(_, len)| len == 0);

        result
    }

    #[inline]
    fn next_back(&mut self) -> Option<&'a JavaStr> {
        if self.finished {
            return None;
        }

        if !self.allow_trailing_empty {
            self.allow_trailing_empty = true;
            match self.next_back() {
                Some(elt) if !elt.is_empty() => return Some(elt),
                _ => {
                    if self.finished {
                        return None;
                    }
                }
            }
        }

        match self.next_match_back() {
            Some((index, len)) => unsafe {
                // SAFETY: pattern guarantees valid indices
                let elt = self.haystack.get_unchecked(index + len..self.end);
                self.end = index;
                Some(elt)
            },
            None => unsafe {
                // SAFETY: `self.start` and `self.end` always lie on unicode boundaries.
                self.finished = true;
                Some(self.haystack.get_unchecked(self.start..self.end))
            },
        }
    }

    #[inline]
    fn next_back_inclusive(&mut self) -> Option<&'a JavaStr> {
        if self.finished {
            return None;
        }

        if !self.allow_trailing_empty {
            self.allow_trailing_empty = true;
            match self.next_back_inclusive() {
                Some(elt) if !elt.is_empty() => return Some(elt),
                _ => {
                    if self.finished {
                        return None;
                    }
                }
            }
        }

        match self.next_match_back() {
            Some((index, len)) => {
                // SAFETY: pattern guarantees valid indices
                let elt = unsafe { self.haystack.get_unchecked(index + len..self.end) };
                self.end = index + len;
                Some(elt)
            }
            None => {
                self.finished = true;
                // SAFETY: `self.start` and `self.end` always lie on unicode boundaries.
                Some(unsafe { self.haystack.get_unchecked(self.start..self.end) })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Split<'a, P> {
    inner: SplitHelper<'a, P>,
}

impl<'a, P> Split<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P) -> Self {
        Split {
            inner: SplitHelper::new(haystack, pat, true),
        }
    }
}

impl<'a, P> Iterator for Split<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, P> DoubleEndedIterator for Split<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, P> FusedIterator for Split<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct RSplit<'a, P> {
    inner: SplitHelper<'a, P>,
}

impl<'a, P> RSplit<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P) -> Self {
        RSplit {
            inner: SplitHelper::new(haystack, pat, true),
        }
    }
}

impl<'a, P> Iterator for RSplit<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, P> DoubleEndedIterator for RSplit<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, P> FusedIterator for RSplit<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct SplitTerminator<'a, P> {
    inner: SplitHelper<'a, P>,
}

impl<'a, P> SplitTerminator<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P) -> Self {
        SplitTerminator {
            inner: SplitHelper::new(haystack, pat, false),
        }
    }
}

impl<'a, P> Iterator for SplitTerminator<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, P> DoubleEndedIterator for SplitTerminator<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, P> FusedIterator for SplitTerminator<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct RSplitTerminator<'a, P> {
    inner: SplitHelper<'a, P>,
}

impl<'a, P> RSplitTerminator<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P) -> Self {
        RSplitTerminator {
            inner: SplitHelper::new(haystack, pat, false),
        }
    }
}

impl<'a, P> Iterator for RSplitTerminator<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, P> DoubleEndedIterator for RSplitTerminator<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, P> FusedIterator for RSplitTerminator<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct SplitInclusive<'a, P> {
    inner: SplitHelper<'a, P>,
}

impl<'a, P> SplitInclusive<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P) -> Self {
        SplitInclusive {
            inner: SplitHelper::new(haystack, pat, false),
        }
    }
}

impl<'a, P> Iterator for SplitInclusive<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_inclusive()
    }
}

impl<'a, P> DoubleEndedIterator for SplitInclusive<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back_inclusive()
    }
}

impl<'a, P> FusedIterator for SplitInclusive<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct SplitN<'a, P> {
    inner: SplitHelper<'a, P>,
    count: usize,
}

impl<'a, P> SplitN<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P, count: usize) -> Self {
        SplitN {
            inner: SplitHelper::new(haystack, pat, true),
            count,
        }
    }
}

impl<'a, P> Iterator for SplitN<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.count {
            0 => None,
            1 => {
                self.count = 0;
                self.inner.get_end()
            }
            _ => {
                self.count -= 1;
                self.inner.next()
            }
        }
    }
}

impl<'a, P> FusedIterator for SplitN<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct RSplitN<'a, P> {
    inner: SplitHelper<'a, P>,
    count: usize,
}

impl<'a, P> RSplitN<'a, P>
where
    P: JavaStrPattern,
{
    #[inline]
    pub(crate) fn new(haystack: &'a JavaStr, pat: P, count: usize) -> Self {
        RSplitN {
            inner: SplitHelper::new(haystack, pat, true),
            count,
        }
    }
}

impl<'a, P> Iterator for RSplitN<'a, P>
where
    P: JavaStrPattern,
{
    type Item = &'a JavaStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.count {
            0 => None,
            1 => {
                self.count = 0;
                self.inner.get_end()
            }
            _ => {
                self.count -= 1;
                self.inner.next_back()
            }
        }
    }
}

impl<'a, P> FusedIterator for RSplitN<'a, P> where P: JavaStrPattern {}

#[derive(Clone, Debug)]
pub struct SplitAsciiWhitespace<'a> {
    #[allow(clippy::type_complexity)]
    pub(crate) inner: Map<
        Filter<slice::Split<'a, u8, fn(&u8) -> bool>, fn(&&[u8]) -> bool>,
        fn(&[u8]) -> &JavaStr,
    >,
}
delegate!(Iterator for SplitAsciiWhitespace<'a> => &'a JavaStr);
delegate!(DoubleEndedIterator for SplitAsciiWhitespace<'a>);
delegate!(FusedIterator for SplitAsciiWhitespace<'a>);

#[derive(Clone, Debug)]
pub struct SplitWhitespace<'a> {
    #[allow(clippy::type_complexity)]
    pub(crate) inner: Filter<Split<'a, fn(JavaCodePoint) -> bool>, fn(&&JavaStr) -> bool>,
}
delegate!(Iterator for SplitWhitespace<'a> => &'a JavaStr);
delegate!(DoubleEndedIterator for SplitWhitespace<'a>);
delegate!(FusedIterator for SplitWhitespace<'a>);
