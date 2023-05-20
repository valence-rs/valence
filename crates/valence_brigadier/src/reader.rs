use std::str::Chars;

/// Contains basic methods for reading string in brigadier
#[derive(Clone, Copy, Debug)]
pub struct StrReader<'a> {
    pub str: &'a str,
    /// Cursor is the index of `byte` not `char`
    cursor: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EndFilterResult {
    /// Continues reading
    Continue,
    /// Moves cursor back to the end and doesn't include it into a result
    EndExclude,
    /// Doesn't move a cursor back and doesn't include it into a result
    EndInclude,
    /// Doesn't move a cursor back and does include it into a result
    EndIncludeResult,
}

impl<'a> StrReader<'a> {
    pub fn new(str: &'a str) -> Self {
        Self { str, cursor: 0 }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// # Safety
    /// Cursor should be a valid char begin in the utf-8 representation of str.
    pub unsafe fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    pub fn read_until_filter<const BACKSLASH: bool, const END_STR_IS_END: bool>(
        &mut self,
        mut is_end_filter: impl FnMut(char) -> EndFilterResult,
    ) -> Option<&'a str> {
        let begin = self.cursor;
        let mut skip_next = false;
        let mut next = self.next_char();
        let offset = loop {
            if !skip_next {
                match next {
                    Some(ch) => {
                        if BACKSLASH && ch == '\\' {
                            skip_next = true;
                        } else {
                            match is_end_filter(ch) {
                                EndFilterResult::Continue => {}
                                EndFilterResult::EndExclude => {
                                    self.cursor -= ch.len_utf8();
                                    break 0;
                                }
                                EndFilterResult::EndInclude => {
                                    break ch.len_utf8();
                                }
                                EndFilterResult::EndIncludeResult => {
                                    break 0;
                                }
                            }
                        }
                    }
                    None => {
                        if END_STR_IS_END {
                            break 0;
                        } else {
                            return None;
                        }
                    }
                }
            } else {
                skip_next = false;
            }
            next = self.next_char();
        };

        self.str_from_to(begin, self.cursor - offset)
    }

    pub fn read_escaped_until_filter<const END_STR_IS_END: bool>(
        &mut self,
        mut is_end_filter: impl FnMut(char) -> EndFilterResult,
    ) -> Option<String> {
        let mut skip_next = false;
        let mut next = self.next_char();
        let mut str = String::new();
        loop {
            match next {
                Some(ch) => {
                    if !skip_next {
                        if ch == '\\' {
                            skip_next = true;
                        } else {
                            match is_end_filter(ch) {
                                EndFilterResult::Continue => str.push(ch),
                                EndFilterResult::EndInclude => {
                                    break;
                                }
                                EndFilterResult::EndExclude => {
                                    self.cursor -= ch.len_utf8();
                                    break;
                                }
                                EndFilterResult::EndIncludeResult => {
                                    str.push(ch);
                                    break;
                                }
                            }
                        }
                    } else {
                        skip_next = false;
                        str.push(ch);
                    }
                }
                None => {
                    return None;
                }
            }
            next = self.next_char();
        }
        Some(str)
    }

    /// Reads unquoted str
    /// ```md
    ///  some unquoted
    ///  ----
    /// ```
    /// Reads only underlined
    pub fn read_unquoted_str(&mut self) -> Option<&'a str> {
        self.read_until_filter::<false, true>(|ch| match ch {
            '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' | '-' | '.' | '+' => EndFilterResult::Continue,
            _ => EndFilterResult::EndExclude,
        })
    }

    /// Reads quoted str which is already opened
    /// ```md
    /// "some quoted" something next
    ///  ------------
    /// ```
    /// Reads only underlined
    pub fn read_quoted_str(&mut self) -> Option<&'a str> {
        self.read_until_filter::<true, false>(|ch| match ch {
            '"' | '\'' => EndFilterResult::EndInclude,
            _ => EndFilterResult::Continue,
        })
    }

    pub fn read_escaped_quoted_str(&mut self) -> Option<String> {
        self.read_escaped_until_filter::<false>(|ch| match ch {
            '"' | '\'' => EndFilterResult::EndInclude,
            _ => EndFilterResult::Continue,
        })
    }

    // TODO do it without an iterator
    pub(crate) fn chars(&self) -> Option<Chars<'a>> {
        self.str.get(self.cursor..).map(|str| str.chars())
    }

    pub fn next_char(&mut self) -> Option<char> {
        let res = self.peek_char();
        if let Some(ch) = res {
            self.cursor += ch.len_utf8()
        }
        res
    }

    pub fn skip_chars(&mut self, count: usize) {
        for _ in 0..count {
            self.next_char();
        }
    }

    pub fn peek_char(&self) -> Option<char> {
        self.chars().and_then(|mut it| it.next())
    }

    pub fn peek_char_offset(&self, offset: usize) -> Option<char> {
        let mut reader = *self;
        for _ in 0..offset {
            reader.next_char();
        }
        reader.peek_char()
    }

    pub fn skip_only(&mut self, ch: char) -> Option<()> {
        if self.peek_char() == Some(ch) {
            self.next_char();
            Some(())
        } else {
            None
        }
    }

    pub fn skip_recursive_only(&mut self, ch: char) {
        while self.skip_only(ch).is_some() {}
    }

    pub fn used_str(&self) -> &'a str {
        self.str.get(0..self.cursor).unwrap()
    }

    pub fn remaining_str(&self) -> &'a str {
        self.str.get(self.cursor..).unwrap()
    }

    pub fn cursor_to_end(&mut self) {
        self.cursor = self.str.as_bytes().len();
    }

    pub fn read_num<const FLOAT: bool>(&mut self) -> Option<&'a str> {
        let mut float_char = false;
        self.read_until_filter::<false, true>(|ch| match ch {
            '0'..='9' | '+' | '-' => EndFilterResult::Continue,
            '.' if FLOAT && !float_char => {
                float_char = true;
                EndFilterResult::Continue
            }
            _ => EndFilterResult::EndExclude,
        })
    }

    pub fn str_from_to(&self, begin: usize, end: usize) -> Option<&'a str> {
        self.str.get(begin..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn read_str_test() {
        let mut reader = StrReader::new(r#"some "text goes here" 2 "hello \"" "3"#);

        assert_eq!(reader.read_unquoted_str(), Some("some"));
        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.next_char(), Some('"'));
        assert_eq!(reader.read_quoted_str(), Some("text goes here"));
        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.read_unquoted_str(), Some("2"));
        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.next_char(), Some('"'));
        assert_eq!(reader.read_escaped_quoted_str(), Some("hello \"".into()));
        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.next_char(), Some('"'));
        assert_eq!(reader.read_quoted_str(), None);
        assert_eq!(reader.used_str(), reader.str);
        assert_eq!(reader.remaining_str(), "");
    }
}
