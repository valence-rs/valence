use crate::p_try;
use crate::parser::ParsingResult;
use crate::reader::StrReader;

/// Parses string like:
/// {element_func}, {element_func}... {end}
pub fn parse_array_like<'a, S, E>(
    reader: &mut StrReader<'a>,
    expected_comma_or_end: (S, E),
    end: char,
    mut element_func: impl FnMut(&mut StrReader<'a>) -> ParsingResult<(), S, E>,
) -> ParsingResult<(), S, E> {
    if !reader.skip_char(end) {
        loop {
            reader.skip_char(' ');
            p_try!(element_func(reader));
            let begin = reader.cursor();
            reader.skip_char(' ');
            match reader.peek_char() {
                Some(',') => {
                    reader.next_char();
                }
                Some(ch) if ch == end => {
                    reader.next_char();
                    break;
                }
                Some(ch) => {
                    let end = begin + ch;
                    return ParsingResult {
                        suggestions: Some((begin..end, expected_comma_or_end.0)),
                        result: Err((begin..end, expected_comma_or_end.1)),
                    };
                }
                None => {
                    return ParsingResult {
                        suggestions: Some((begin..begin, expected_comma_or_end.0)),
                        result: Err((begin..begin, expected_comma_or_end.1)),
                    }
                }
            }
        }
    }

    ParsingResult::ok()
}
