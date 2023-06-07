use std::ops::{Add, AddAssign};
use std::str::{Chars, FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StrCursor {
    pub bytes: usize,
    pub chars: usize,
}

impl StrCursor {
    pub const fn new(bytes: usize, chars: usize) -> Self {
        Self { bytes, chars }
    }

    #[cfg(test)]
    pub fn new_str(str: &str) -> Self {
        Self::new(str.len(), str.chars().count())
    }

    #[cfg(test)]
    pub fn new_range(begin: &str, taken: &str) -> std::ops::Range<Self> {
        let l = Self::new_str(begin);
        let mut r = Self::new_str(taken);
        r.bytes += l.bytes;
        r.chars += l.chars;
        l..r
    }
}

impl Add<char> for StrCursor {
    type Output = Self;

    fn add(mut self, rhs: char) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<char> for StrCursor {
    fn add_assign(&mut self, rhs: char) {
        self.bytes += rhs.len_utf8();
        self.chars += 1;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StrReader<'a> {
    str: &'a str,
    cursor: StrCursor,
}

impl<'a> From<&'a str> for StrReader<'a> {
    fn from(value: &'a str) -> Self {
        StrReader::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrFilter {
    Continue,
    EndStrInclude,
    EndInclude,
    EndExclude,
}

impl<'a> StrReader<'a> {
    pub const fn new(str: &'a str) -> Self {
        Self {
            str,
            cursor: StrCursor::new(0, 0),
        }
    }

    pub fn str(&self) -> &'a str {
        self.str
    }

    /// # Safety
    /// Given cursor should be valid
    pub unsafe fn set_cursor(&mut self, cursor: StrCursor) {
        self.cursor = cursor;
    }

    /// # Safety
    /// Given bytes should be valid for current cursor
    pub unsafe fn move_cursor_right(&mut self, bytes: usize) {
        let new_bytes = self.cursor.bytes + bytes;
        self.cursor.chars += unsafe { self.str.get_unchecked(self.cursor.bytes..new_bytes) }
            .chars()
            .count();
        self.cursor.bytes = new_bytes;
    }

    pub fn cursor(&self) -> StrCursor {
        self.cursor
    }

    fn chars(&self) -> Chars<'a> {
        self.remaining_str().chars()
    }

    pub fn peek_char(&self) -> Option<char> {
        self.chars().next()
    }

    pub fn peek_offset_char(&self, offset: usize) -> Option<char> {
        let mut iter = self.chars();
        for _ in 0..offset {
            iter.next();
        }
        iter.next()
    }

    pub fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char();
        if let Some(ch) = ch {
            self.cursor += ch;
        }

        ch
    }

    pub fn skip_str_filtered(&mut self, mut filter: impl FnMut(char) -> StrFilter) -> StrCursor {
        loop {
            match self.peek_char() {
                Some(ch) => match filter(ch) {
                    StrFilter::Continue => {
                        self.cursor += ch;
                    }
                    StrFilter::EndExclude => {
                        break self.cursor;
                    }
                    StrFilter::EndStrInclude => {
                        self.cursor += ch;
                        break self.cursor;
                    }
                    StrFilter::EndInclude => {
                        let end = self.cursor;
                        self.cursor += ch;
                        break end;
                    }
                },
                None => {
                    break self.cursor;
                }
            }
        }
    }

    pub fn read_str_filtered(&mut self, filter: impl FnMut(char) -> StrFilter) -> &'a str {
        let begin = self.cursor();

        let end = self.skip_str_filtered(filter);

        // SAFETY: begin and end are valid cursors
        unsafe { self.str.get_unchecked(begin.bytes..end.bytes) }
    }

    pub fn skip_escaped_str_filtered(&mut self, mut filter: impl FnMut(char) -> StrFilter) -> bool {
        let mut skip_next = false;
        let mut ended = false;
        self.skip_str_filtered(|ch| {
            if ch == '\\' {
                skip_next = true;
                StrFilter::Continue
            } else if !skip_next {
                let filter = filter(ch);
                if filter != StrFilter::Continue {
                    ended = true;
                }
                filter
            } else {
                skip_next = false;
                StrFilter::Continue
            }
        });
        ended
    }

    pub fn read_escaped_str_filtered(
        &mut self,
        mut filter: impl FnMut(char) -> StrFilter,
    ) -> Option<String> {
        let mut result = String::new();
        let mut skip_next = false;

        loop {
            match self.peek_char() {
                Some(ch) => {
                    if ch == '\\' && !skip_next {
                        skip_next = true;
                        self.cursor += ch;
                    } else {
                        match filter(ch) {
                            StrFilter::Continue => {
                                self.cursor += ch;
                                result.push(ch);
                            }
                            StrFilter::EndStrInclude => {
                                self.cursor += ch;
                                result.push(ch);
                                break;
                            }
                            StrFilter::EndInclude => {
                                self.cursor += ch;
                                break;
                            }
                            StrFilter::EndExclude => {
                                break;
                            }
                        }
                        skip_next = false;
                    }
                }
                None => {
                    return None;
                }
            }
        }

        Some(result)
    }

    pub fn read_unquoted_str(&mut self) -> &'a str {
        self.read_str_filtered(|ch| match ch {
            '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' | '-' | '.' | '+' => StrFilter::Continue,
            _ => StrFilter::EndExclude,
        })
    }

    pub fn read_delimitted_str(&mut self) -> &'a str {
        self.read_str_filtered(|ch| match ch {
            ' ' => StrFilter::EndExclude,
            _ => StrFilter::Continue,
        })
    }

    pub fn read_ident_str(&mut self) -> (Option<&'a str>, &'a str) {
        let mut left = false;
        let result = self.read_str_filtered(|ch| match ch {
            '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' | '-' | '.' | '+' => StrFilter::Continue,
            ':' => {
                left = true;
                StrFilter::EndInclude
            }
            _ => StrFilter::EndExclude,
        });

        if left {
            (Some(result), self.read_unquoted_str())
        } else {
            (None, result)
        }
    }

    pub fn read_started_quoted_str(&mut self) -> Option<String> {
        self.read_escaped_str_filtered(|ch| match ch {
            '"' | '\'' => StrFilter::EndInclude,
            _ => StrFilter::Continue,
        })
    }

    pub fn skip_started_quoted_str(&mut self) -> bool {
        self.skip_escaped_str_filtered(|ch| match ch {
            '"' | '\'' => StrFilter::EndInclude,
            _ => StrFilter::Continue,
        })
    }

    pub fn read_int_str(&mut self) -> &'a str {
        let begin = self.cursor();
        if let Some('+') | Some('-') = self.peek_char() {
            self.next_char();
        }
        let end = self.skip_str_filtered(|ch| match ch {
            '0'..='9' => StrFilter::Continue,
            _ => StrFilter::EndExclude,
        });
        unsafe { self.str.get_unchecked(begin.bytes..end.bytes) }
    }

    pub fn read_float_str(&mut self) -> (&'a str, bool) {
        let begin = self.cursor();
        if let Some('+') | Some('-') = self.peek_char() {
            self.next_char();
        }
        let mut fp = false;

        loop {
            let ch = self.peek_char();
            match ch {
                Some('0'..='9') => {
                    self.next_char();
                }
                Some('.') if fp => {
                    break;
                }
                Some('.') => {
                    fp = true;
                    if !matches!(self.peek_offset_char(1), Some('0'..='9')) {
                        break;
                    }
                    self.skip_next_chars(2);
                }
                _ => {
                    break;
                }
            }
        }

        (
            // SAFETY: cursors are always valid
            unsafe { self.str.get_unchecked(begin.bytes..self.cursor().bytes) },
            fp,
        )
    }

    pub fn read_int<T: FromStr>(&mut self) -> Result<T, <T as FromStr>::Err> {
        self.read_int_str().parse()
    }

    pub fn read_float<T: FromStr>(&mut self) -> Result<T, <T as FromStr>::Err> {
        self.read_float_str().0.parse()
    }

    pub fn remaining_str(&self) -> &'a str {
        // SAFETY: cursor is always valid
        unsafe { self.str.get_unchecked(self.cursor.bytes..) }
    }

    pub fn used_str(&self) -> &'a str {
        // SAFETY: cursor is always valid
        unsafe { self.str.get_unchecked(..self.cursor.bytes) }
    }

    pub fn is_ended(&self) -> bool {
        self.cursor.bytes == self.str.len()
    }

    pub fn to_end(&mut self) {
        // SAFETY: cursor is always valid
        unsafe { self.move_cursor_right(self.str.len() - self.cursor.bytes) }
    }

    pub fn skip_char(&mut self, ch: char) -> bool {
        if self.peek_char() == Some(ch) {
            self.next_char();
            true
        } else {
            false
        }
    }

    pub fn skip_next_chars(&mut self, count: usize) {
        for _ in 0..count {
            self.next_char();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_filtered_test() {
        let mut reader = StrReader::new(r#"aaa "32" "64"#);

        assert_eq!(
            reader.read_str_filtered(|ch| match ch {
                ' ' => StrFilter::EndExclude,
                _ => StrFilter::Continue,
            }),
            "aaa"
        );

        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.next_char(), Some('"'));

        assert_eq!(
            reader.read_escaped_str_filtered(|ch| match ch {
                '"' => StrFilter::EndInclude,
                _ => StrFilter::Continue,
            }),
            Some("32".to_string())
        );

        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.next_char(), Some('"'));

        assert_eq!(
            reader.read_escaped_str_filtered(|ch| match ch {
                '"' => StrFilter::EndExclude,
                _ => StrFilter::Continue,
            }),
            None
        );
    }

    #[test]
    fn skip_filtered_str() {
        let mut reader = StrReader::new(r#"a thing""#);

        assert!(reader.skip_escaped_str_filtered(|ch| match ch {
            '"' => StrFilter::EndInclude,
            _ => StrFilter::Continue,
        }));

        assert!(reader.is_ended())
    }
}
