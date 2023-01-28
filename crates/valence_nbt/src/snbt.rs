use std::{
    error::Error,
    fmt::{Display, Formatter},
    iter::Peekable,
    str::Chars,
};

use crate::{tag::Tag, Compound, List, Value};

const STRING_MAX_LEN: usize = 32767;
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SNBTErrorType {
    ReachEndOfStream,
    InvalidEscapeSequence,
    EmptyKeyInCompound,
    ExpectColon,
    ExpectValue,
    ExpectComma,
    WrongTypeInArray,
    DifferentTypesInList,
    LongString,
    TrailingData,
}

impl Display for SNBTErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use SNBTErrorType::*;
        match self {
            ReachEndOfStream => write!(f, "Reach end of stream"),
            InvalidEscapeSequence => write!(f, "Invalid escape sequence"),
            EmptyKeyInCompound => write!(f, "Empty key in compound"),
            ExpectColon => write!(f, "Expect colon"),
            ExpectValue => write!(f, "Expect value"),
            ExpectComma => write!(f, "Expect comma"),
            WrongTypeInArray => write!(f, "Wrong type in array"),
            DifferentTypesInList => write!(f, "Different types in list"),
            LongString => write!(f, "Long string"),
            TrailingData => write!(f, "Extra data after end"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct SNBTError {
    pub error_type: SNBTErrorType,
    pub line: usize,
    pub column: usize,
}

impl SNBTError {
    pub fn new(error_type: SNBTErrorType, line: usize, column: usize) -> Self {
        Self {
            error_type,
            line,
            column,
        }
    }
}

impl Display for SNBTError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "@ {},{}: {}", self.line, self.column, self.error_type)
    }
}

impl Error for SNBTError {}

type Result<T> = std::result::Result<T, SNBTError>;

pub struct SNBTReader<'a> {
    line: usize,
    column: usize,
    pub index: usize,
    iter: Peekable<Chars<'a>>,
    pushed_back: Option<char>,
}

impl<'a> SNBTReader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            line: 1,
            column: 1,
            index: 0,
            iter: input.chars().peekable(),
            pushed_back: None,
        }
    }

    fn make_error(&self, error_type: SNBTErrorType) -> SNBTError {
        SNBTError::new(error_type, self.line, self.column)
    }

    fn peek(&mut self) -> Result<char> {
        if let Some(c) = self.pushed_back {
            Ok(c)
        } else {
            self.iter
                .peek()
                .map(|c| *c)
                .ok_or_else(|| self.make_error(SNBTErrorType::ReachEndOfStream))
        }
    }

    fn next(&mut self) {
        if let Some(_) = self.pushed_back {
            self.pushed_back = None;
            return;
        }
        let result = self.iter.next();
        if let Some(c) = result {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.index += c.len_utf8();
        }
    }

    /// Push back a char, only one char can be pushed back
    fn push_back(&mut self, c: char) {
        if c == '\n' {
            self.line -= 1;
            self.column = 1;
        } else {
            self.column -= 1;
        }
        self.index -= c.len_utf8();
        match self.pushed_back {
            Some(_) => panic!("Can't push back two chars"),
            None => self.pushed_back = Some(c),
        };
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Ok(c) if c.is_whitespace() => self.next(),
                _ => break,
            };
        }
    }

    fn read_string(&mut self) -> Result<String> {
        let first = self.peek()?;
        let str = match first {
            '\"' | '\'' => self.read_quoted_string(),
            _ => self.read_unquoted_string(),
        }?;
        if str.len() > STRING_MAX_LEN {
            return Err(self.make_error(SNBTErrorType::LongString));
        }
        Ok(str)
    }

    fn read_unquoted_string(&mut self) -> Result<String> {
        let mut result = String::new();
        loop {
            let input = self.peek();
            match input {
                Ok('a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '+' | '.') => {
                    result.push(input?);
                    self.next();
                }
                _ => break,
            }
        }
        Ok(result)
    }

    fn read_quoted_string(&mut self) -> Result<String> {
        let quote = self.peek()?;
        self.next();
        let mut result = String::new();
        loop {
            let input = self.peek();
            match input {
                Ok(c) if c == quote => {
                    self.next();
                    break;
                }
                Ok('\\') => {
                    self.next();
                    let escape = self.peek()?;
                    if escape == quote || escape == '\\' {
                        result.push(escape);
                    } else {
                        return Err(self.make_error(SNBTErrorType::InvalidEscapeSequence));
                    }
                    self.next();
                }
                Ok(c) => {
                    result.push(c);
                    self.next();
                }
                Err(e) => return Err(e),
            }
        }
        if result.len() > STRING_MAX_LEN {
            return Err(self.make_error(SNBTErrorType::LongString));
        }
        Ok(result)
    }

    fn parse_compound(&mut self) -> Result<Compound> {
        self.next();
        self.skip_whitespace();
        let mut cpd = Compound::new();
        while self.peek()? != '}' {
            let key = self.read_string()?;
            self.skip_whitespace();
            if key.len() == 0 {
                return Err(self.make_error(SNBTErrorType::EmptyKeyInCompound));
            }
            if self.peek()? != ':' {
                return Err(self.make_error(SNBTErrorType::ExpectColon));
            }
            self.next();
            self.skip_whitespace();
            let value = self.parse_element()?;
            self.skip_whitespace();
            if self.peek()? == ',' {
                self.next();
                self.skip_whitespace();
            } else if self.peek()? != '}' {
                return Err(self.make_error(SNBTErrorType::ExpectComma));
            }
            cpd.insert(key, value);
        }
        self.next();
        Ok(cpd)
    }

    fn continue_parse_list(&mut self) -> Result<List> {
        self.skip_whitespace();
        let mut list = vec![];
        let mut element_type = Tag::End;
        while self.peek()? != ']' {
            let value = self.parse_element()?;
            self.skip_whitespace();
            if element_type == Tag::End {
                element_type = value.get_type();
            } else if value.get_type() != element_type {
                return Err(self.make_error(SNBTErrorType::DifferentTypesInList));
            }
            if self.peek()? == ',' {
                self.next();
                self.skip_whitespace();
            } else if self.peek()? != ']' {
                return Err(self.make_error(SNBTErrorType::ExpectComma));
            }
            list.push(value);
        }
        self.next();

        // Since the type of elements is known, feel free to unwrap them
        match element_type {
            Tag::End => Ok(List::End),
            Tag::Byte => Ok(List::Byte(
                list.into_iter().map(|v| v.into_byte().unwrap()).collect(),
            )),
            Tag::Short => Ok(List::Short(
                list.into_iter().map(|v| v.into_short().unwrap()).collect(),
            )),
            Tag::Int => Ok(List::Int(
                list.into_iter().map(|v| v.into_int().unwrap()).collect(),
            )),
            Tag::Long => Ok(List::Long(
                list.into_iter().map(|v| v.into_long().unwrap()).collect(),
            )),
            Tag::Float => Ok(List::Float(
                list.into_iter().map(|v| v.into_float().unwrap()).collect(),
            )),
            Tag::Double => Ok(List::Double(
                list.into_iter().map(|v| v.into_double().unwrap()).collect(),
            )),
            Tag::String => Ok(List::String(
                list.into_iter().map(|v| v.into_string().unwrap()).collect(),
            )),
            Tag::ByteArray => Ok(List::ByteArray(
                list.into_iter()
                    .map(|v| v.into_byte_array().unwrap())
                    .collect(),
            )),
            Tag::List => Ok(List::List(
                list.into_iter().map(|v| v.into_list().unwrap()).collect(),
            )),
            Tag::Compound => Ok(List::Compound(
                list.into_iter()
                    .map(|v| v.into_compound().unwrap())
                    .collect(),
            )),
            Tag::IntArray => Ok(List::IntArray(
                list.into_iter()
                    .map(|v| v.into_int_array().unwrap())
                    .collect(),
            )),
            Tag::LongArray => Ok(List::LongArray(
                list.into_iter()
                    .map(|v| v.into_long_array().unwrap())
                    .collect(),
            )),
        }
    }

    fn parse_list_like(&mut self) -> Result<Value> {
        self.next();
        let type_char = self.peek()?;
        let etype = match type_char {
            'B' => Tag::Byte,
            'I' => Tag::Int,
            'L' => Tag::Long,
            _ => return self.continue_parse_list().map(|l| l.into()),
        };
        self.next();
        if self.peek()? != ';' {
            self.push_back(type_char);
            return self.continue_parse_list().map(|l| l.into());
        }
        self.next();
        self.skip_whitespace();
        let mut values = vec![];
        while self.peek()? != ']' {
            let value = self.parse_element()?;
            if value.get_type() != etype {
                return Err(self.make_error(SNBTErrorType::WrongTypeInArray));
            }
            values.push(value);
            self.skip_whitespace();
            if self.peek()? == ',' {
                self.next();
                self.skip_whitespace();
            } else if self.peek()? != ']' {
                return Err(self.make_error(SNBTErrorType::ExpectComma));
            }
        }
        self.next();
        match etype {
            Tag::Byte => Ok(Value::ByteArray(
                values.into_iter().map(|v| v.into_byte().unwrap()).collect(),
            )),
            Tag::Int => Ok(Value::IntArray(
                values.into_iter().map(|v| v.into_int().unwrap()).collect(),
            )),
            Tag::Long => Ok(Value::LongArray(
                values.into_iter().map(|v| v.into_long().unwrap()).collect(),
            )),
            _ => unreachable!(),
        }
    }

    fn parse_primitive(&mut self) -> Result<Value> {
        macro_rules! try_ret {
            // Try possible solution until one works
            ($v:expr) => {{
                match $v {
                    Ok(v) => return Ok(v.into()),
                    Err(_) => (),
                }
            }};
        }
        let target = self.read_unquoted_string()?;
        match target
            .bytes()
            .last()
            .ok_or_else(|| self.make_error(SNBTErrorType::ExpectValue))?
        {
            b'b' | b'B' => try_ret!(target[..target.len() - 1].parse::<i8>()),
            b's' | b'S' => try_ret!(target[..target.len() - 1].parse::<i16>()),
            b'l' | b'L' => try_ret!(target[..target.len() - 1].parse::<i64>()),
            b'f' | b'F' => try_ret!(target[..target.len() - 1].parse::<f32>()),
            b'd' | b'D' => try_ret!(target[..target.len() - 1].parse::<f64>()),
            _ => (),
        }
        match target.as_str() {
            "true" => return Ok(Value::Byte(1)),
            "false" => return Ok(Value::Byte(0)),
            _ => {
                try_ret!(target.parse::<i32>());
                try_ret!(target.parse::<f64>());
            }
        };
        if target.len() > STRING_MAX_LEN {
            return Err(self.make_error(SNBTErrorType::LongString));
        }
        Ok(Value::String(target))
    }

    /// Read the next element in the SNBT string.
    /// [`SNBTErrorType::TrailingData`] is impossible to be returned since it doesnot consider it's an error.
    pub fn parse_element(&mut self) -> Result<Value> {
        self.skip_whitespace();
        match self.peek()? {
            '{' => self.parse_compound().map(|c| c.into()),
            '[' => self.parse_list_like(),
            '"' | '\'' => self.read_quoted_string().map(|s| s.into()),
            _ => self.parse_primitive(),
        }
    }

    pub fn read(&mut self) -> Result<Value> {
        let value = self.parse_element()?;
        self.skip_whitespace();
        if self.peek().is_ok() {
            return Err(self.make_error(SNBTErrorType::TrailingData));
        }
        Ok(value)
    }

    /// Get the number of bytes readed.
    /// It's useful when you want to read a SNBT string from an command argument since there may be trailing data.
    pub fn bytes_readed(&self) -> usize {
        self.index
    }

    /// Parse a string in SNBT format into a `Value`.
    /// Assert that the string has no trailing data.
    /// SNBT is quite similar to JSON, but with some differences.
    /// See [the wiki](https://minecraft.gamepedia.com/NBT_format#SNBT_format) for more information.
    /// # Example
    /// ```
    /// use nbt::Value;
    /// use nbt::SNBTReader;
    /// let value = SNBTReader::from_snbt("1f").unwrap();
    /// assert_eq!(value, Value::Float(1.0));
    /// ```
    pub fn from_snbt(snbt: &str) -> Result<Value> {
        SNBTReader::new(snbt).read()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse() {
        let str = r#"
			{
				foo: 1,
				'bar': 1.0,
				"baz": 1.0f,
				"hello'": "hello world",
				"world": "hello\"world",
				1.5f: 1.5d,
				3b: 2f,
				bool: false,
				more: {
					iarr: [I; 1, 2, 3],
					larr: [L; 1L, 2L, 3L],
				},
				empty: [Bibabo ],
			}
		"#;
        let value = SNBTReader::from_snbt(str).unwrap();
        let cpd = value.as_compound().unwrap();
        assert_eq!(*cpd.get("foo").unwrap().as_int().unwrap(), 1);
        assert_eq!(*cpd.get("bar").unwrap().as_double().unwrap(), 1.0);
        assert_eq!(*cpd.get("baz").unwrap().as_float().unwrap(), 1.0);
        assert_eq!(
            *cpd.get("hello'").unwrap().as_string().unwrap(),
            "hello world"
        );
        assert_eq!(
            *cpd.get("world").unwrap().as_string().unwrap(),
            "hello\"world"
        );
        assert_eq!(*cpd.get("1.5f").unwrap().as_double().unwrap(), 1.5);
        assert_eq!(*cpd.get("3b").unwrap().as_float().unwrap(), 2.0);
        assert_eq!(*cpd.get("bool").unwrap().as_byte().unwrap(), 0);
        let more = cpd.get("more").unwrap().as_compound().unwrap();
        assert_eq!(
            *more.get("iarr").unwrap().as_int_array().unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(
            *more.get("larr").unwrap().as_long_array().unwrap(),
            vec![1, 2, 3]
        );
        println!("{:?}", more);
        let List::String(list) = cpd.get("empty").unwrap().as_list().unwrap() else { panic!() };
        assert_eq!(list[0], "Bibabo");
        assert_eq!(
            SNBTReader::from_snbt("\"\\n\"").unwrap_err().error_type,
            SNBTErrorType::InvalidEscapeSequence
        );
        assert_eq!(
            SNBTReader::from_snbt("[L; 1]").unwrap_err().error_type,
            SNBTErrorType::WrongTypeInArray
        );
        assert_eq!(
            SNBTReader::from_snbt("[L; 1L, 2L, 3L")
                .unwrap_err()
                .error_type,
            SNBTErrorType::ReachEndOfStream
        );
        assert_eq!(
            SNBTReader::from_snbt("[L; 1L, 2L, 3L,]dewdwe")
                .unwrap_err()
                .error_type,
            SNBTErrorType::TrailingData
        );
        assert_eq!(
            SNBTReader::from_snbt("{ foo: }").unwrap_err().error_type,
            SNBTErrorType::ExpectValue
        );
        assert_eq!(
            SNBTReader::from_snbt("{ {}, }").unwrap_err().error_type,
            SNBTErrorType::EmptyKeyInCompound
        );
        assert_eq!(
            SNBTReader::from_snbt("{ foo 1 }").unwrap_err().error_type,
            SNBTErrorType::ExpectColon
        );
        assert_eq!(
            SNBTReader::from_snbt("{ foo: 1 bar: 2 }")
                .unwrap_err()
                .error_type,
            SNBTErrorType::ExpectComma
        );
        assert_eq!(
            SNBTReader::from_snbt("[{}, []]").unwrap_err().error_type,
            SNBTErrorType::DifferentTypesInList
        );
        assert_eq!(
            SNBTReader::from_snbt(&String::from_utf8(vec![b'e'; 32768]).unwrap())
                .unwrap_err()
                .error_type,
            SNBTErrorType::LongString
        );
    }
}
