use std::ops::{Add, AddAssign, Range, Sub, SubAssign};
use std::str::Chars;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StrCursor {
    bytes: usize,
    chars: usize,
}

impl AddAssign<char> for StrCursor {
    fn add_assign(&mut self, rhs: char) {
        self.bytes += rhs.len_utf8();
        self.chars += 1;
    }
}

impl Add<char> for StrCursor {
    type Output = Self;

    fn add(mut self, rhs: char) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<char> for StrCursor {
    fn sub_assign(&mut self, rhs: char) {
        self.bytes -= rhs.len_utf8();
        self.chars -= 1;
    }
}

impl Sub<char> for StrCursor {
    type Output = Self;

    fn sub(mut self, rhs: char) -> Self::Output {
        self -= rhs;
        self
    }
}

impl StrCursor {
    pub const fn start() -> Self {
        Self { bytes: 0, chars: 0 }
    }

    pub fn chars(&self) -> usize {
        self.chars
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StrSpan {
    begin: StrCursor,
    end: StrCursor,
}

impl StrSpan {
    pub const fn new(begin: StrCursor, end: StrCursor) -> Self {
        debug_assert!(begin.bytes <= end.bytes);
        Self { begin, end }
    }

    pub fn join(self, other: Self) -> Self {
        Self::new(self.begin, other.end)
    }

    pub const fn start() -> Self {
        Self {
            begin: StrCursor::start(),
            end: StrCursor::start(),
        }
    }
}

impl From<Range<StrCursor>> for StrSpan {
    fn from(value: Range<StrCursor>) -> Self {
        Self::new(value.start, value.end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StrLocated<T> {
    pub span: StrSpan,
    pub object: T,
}

impl<T> StrLocated<T> {
    pub const fn new(span: StrSpan, object: T) -> Self {
        Self { span, object }
    }

    pub fn map<T1>(self, func: impl FnOnce(T) -> T1) -> StrLocated<T1> {
        StrLocated {
            span: self.span,
            object: func(self.object),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrReader<'a> {
    str: &'a str,
    cursor: StrCursor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrReaderFilter {
    /// Reader continues read
    Continue,
    /// Reader includes char and stops read
    IncludeStr,
    /// Reader moves cursor but does not include char and stops read
    IncludeCursor,
    /// Reader does not neither move cursor nor include char in str and then
    /// stops read
    Exclude,
}

impl<'a> StrReader<'a> {
    pub const fn new(str: &'a str) -> Self {
        Self {
            str,
            cursor: StrCursor { bytes: 0, chars: 0 },
        }
    }

    /// Returns valid cursor
    pub const fn cursor(&self) -> StrCursor {
        self.cursor
    }

    pub fn to_end(&mut self) {
        self.cursor.chars += self.chars().count();
        self.cursor.bytes = self.str.len();
    }

    /// # Safety
    /// Span should be valid
    pub unsafe fn get_str(&self, span: impl Into<StrSpan>) -> &'a str {
        let span = span.into();
        // SAFETY: function accepts only valid spans
        unsafe { self.str.get_unchecked(span.begin.bytes..span.end.bytes) }
    }

    pub const fn str(&self) -> &'a str {
        self.str
    }

    pub fn remaining_str(&self) -> &'a str {
        // SAFETY: cursor is always valid
        unsafe { self.str.get_unchecked(self.cursor.bytes..) }
    }

    pub fn used_str(&self) -> &'a str {
        // SAFETY: cursor is always valid
        unsafe { self.str.get_unchecked(..self.cursor.bytes) }
    }

    pub fn chars(&self) -> Chars {
        self.remaining_str().chars()
    }

    pub fn peek_char(&self) -> Option<char> {
        self.chars().next()
    }

    pub fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char();
        if let Some(ch) = ch {
            self.cursor += ch;
        }
        ch
    }

    pub fn skip_char(&mut self, ch: char) -> bool {
        if self.peek_char() == Some(ch) {
            self.next_char();
            true
        } else {
            false
        }
    }

    pub fn located<T>(&mut self, func: impl FnOnce(&mut Self) -> T) -> StrLocated<T> {
        let begin = self.cursor();
        let res = func(self);
        StrLocated::new(StrSpan::new(begin, self.cursor()), res)
    }

    pub fn err_located<T, E>(
        &mut self,
        func: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, StrLocated<E>> {
        let begin = self.cursor();
        let res = func(self);
        res.map_err(|e| StrLocated::new(StrSpan::new(begin, self.cursor()), e))
    }

    pub fn span_err_located<T, E>(
        &mut self,
        span: &mut StrSpan,
        func: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, StrLocated<E>> {
        self.span_located(span, func)
            .map_err(|err| StrLocated::new(*span, err))
    }

    pub fn span_located<T>(&mut self, span: &mut StrSpan, func: impl FnOnce(&mut Self) -> T) -> T {
        let begin = self.cursor();
        let res = func(self);
        *span = StrSpan::new(begin, self.cursor());
        res
    }

    /// Skips string using given filter.
    /// ### Returns
    /// The end of string
    pub fn skip_str(&mut self, mut filter: impl FnMut(char) -> StrReaderFilter) -> StrCursor {
        loop {
            let ch = self.peek_char();
            match ch {
                Some(ch) => match filter(ch) {
                    StrReaderFilter::Continue => {
                        self.cursor += ch;
                    }
                    StrReaderFilter::IncludeStr => {
                        self.cursor += ch;
                        break self.cursor;
                    }
                    StrReaderFilter::IncludeCursor => {
                        let end = self.cursor;
                        self.cursor += ch;
                        break end;
                    }
                    StrReaderFilter::Exclude => break self.cursor,
                },
                None => break self.cursor(),
            }
        }
    }

    pub fn read_str(&mut self, filter: impl FnMut(char) -> StrReaderFilter) -> &'a str {
        let begin = self.cursor();
        let end = self.skip_str(filter);
        // SAFETY: begin and end are valid cursors
        unsafe { self.get_str(begin..end) }
    }

    pub fn skip_escaped_str(
        &mut self,
        mut filter: impl FnMut(char) -> StrReaderFilter,
        mut chars: impl FnMut(char),
    ) -> bool {
        let mut next = false;
        let mut last = false;
        self.skip_str(|ch| match (ch, next) {
            ('\\', false) => {
                next = true;
                StrReaderFilter::Continue
            }
            (ch, true) => {
                chars(ch);
                StrReaderFilter::Continue
            }
            (ch, false) => {
                let filter_r = filter(ch);
                if let StrReaderFilter::Continue | StrReaderFilter::IncludeStr = filter_r {
                    chars(ch);
                }
                if filter_r != StrReaderFilter::Continue {
                    last = true;
                }
                filter_r
            }
        });
        last
    }

    pub fn read_escaped_str(
        &mut self,
        filter: impl FnMut(char) -> StrReaderFilter,
    ) -> Option<String> {
        let mut result = String::new();
        match self.skip_escaped_str(filter, |ch| result.push(ch)) {
            true => Some(result),
            false => None,
        }
    }

    pub fn read_unquoted_str(&mut self) -> &'a str {
        self.read_str(|ch| match ch {
            '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' | '-' | '.' | '+' => StrReaderFilter::Continue,
            _ => StrReaderFilter::Exclude,
        })
    }

    pub fn read_delimitted_str(&mut self) -> &'a str {
        self.read_str(|ch| match ch {
            ' ' => StrReaderFilter::Exclude,
            _ => StrReaderFilter::Continue,
        })
    }

    pub fn read_resource_location_str(&mut self) -> &'a str {
        self.read_str(|ch| match ch {
            '0'..='9' | 'a'..='z' | '_' | ':' | '/' | '.' | '-' => StrReaderFilter::Continue,
            _ => StrReaderFilter::Exclude,
        })
    }

    pub fn read_ident_str(&mut self) -> (Option<&'a str>, &'a str) {
        let mut left = false;
        let result = self.read_str(|ch| match ch {
            '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' | '-' | '.' | '+' => StrReaderFilter::Continue,
            ':' => {
                left = true;
                StrReaderFilter::IncludeCursor
            }
            _ => StrReaderFilter::Exclude,
        });

        if left {
            (Some(result), self.read_unquoted_str())
        } else {
            (None, result)
        }
    }

    pub fn read_started_quoted_str(&mut self) -> Option<String> {
        self.read_escaped_str(|ch| match ch {
            '"' | '\'' => StrReaderFilter::IncludeCursor,
            _ => StrReaderFilter::Continue,
        })
    }

    pub fn skip_started_quoted_str(&mut self) -> bool {
        self.skip_escaped_str(
            |ch| match ch {
                '"' | '\'' => StrReaderFilter::IncludeCursor,
                _ => StrReaderFilter::Continue,
            },
            |_| {},
        )
    }

    pub fn read_num_str(&mut self) -> &'a str {
        self.read_str(|ch| match ch {
            '0'..='9' | '+' | '-' | 'e' | '.' => StrReaderFilter::Continue,
            _ => StrReaderFilter::Exclude,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn reader_test() {
        assert_eq!(StrReader::new("hello n").read_unquoted_str(), "hello");
        assert_eq!(
            StrReader::new("minecraft:stick n").read_ident_str(),
            (Some("minecraft"), "stick")
        );
        assert_eq!(
            StrReader::new(r#"hello" n"#).read_started_quoted_str(),
            Some("hello".to_string())
        );
    }
}
