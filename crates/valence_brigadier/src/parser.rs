use std::str::Chars;

#[derive(Clone, Copy, Debug)]
pub struct StrReader<'a> {
    pub str: &'a str,
    cursor: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EndFilterResult {
    Continue,
    EndExclude,
    EndInclude,
}

impl<'a> StrReader<'a> {
    pub fn new(str: &'a str) -> Self {
        Self { str, cursor: 0 }
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
                                    break ch.len_utf8();
                                }
                                EndFilterResult::EndInclude => {
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

        self.cursor -= offset;

        self.str.get(begin..self.cursor)
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
                                    str.push(ch);
                                    break;
                                }
                                EndFilterResult::EndExclude => {
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
            '"' | '\'' => EndFilterResult::EndExclude,
            _ => EndFilterResult::Continue,
        })
    }

    pub fn read_escaped_quoted_str(&mut self) -> Option<String> {
        self.read_escaped_until_filter::<false>(|ch| match ch {
            '"' | '\'' => EndFilterResult::EndExclude,
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

    pub fn peek_char(&self) -> Option<char> {
        self.chars().and_then(|mut it| it.next())
    }

    pub fn skip_only(&mut self, ch: char) -> Option<()> {
        if self.peek_char() == Some(ch) {
            self.next_char();
            Some(())
        } else {
            None
        }
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
        self.read_until_filter::<false, true>(|ch| match ch {
            '0'..='9' | '+' | '-' => EndFilterResult::Continue,
            '.' | ',' if FLOAT => EndFilterResult::Continue,
            _ => EndFilterResult::EndExclude,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn read_str_test() {
        let mut reader = StrReader::new(r#"some "text goes here" 2 "hello \"" "3"#);

        assert_eq!(reader.read_unquoted_str(), Some("some"));
        assert_eq!(reader.next_char(), Some('"'));
        assert_eq!(reader.read_quoted_str(), Some("text goes here"));
        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.read_unquoted_str(), Some("2"));
        assert_eq!(reader.next_char(), Some('"'));
        assert_eq!(reader.read_escaped_quoted_str(), Some("hello \"".into()));
        assert_eq!(reader.next_char(), Some(' '));
        assert_eq!(reader.next_char(), Some('"'));
        assert_eq!(reader.read_quoted_str(), None);
        assert_eq!(reader.used_str(), reader.str);
        assert_eq!(reader.remaining_str(), "");
    }
}
