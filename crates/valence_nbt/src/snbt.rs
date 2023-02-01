use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::str::Chars;

use crate::tag::Tag;
use crate::{Compound, List, Value};

const STRING_MAX_LEN: usize = 32767;
/// Maximum recursion depth to prevent overflowing the call stack.
const MAX_DEPTH: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SnbtErrorKind {
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
    DepthLimitExceeded,
}

impl Display for SnbtErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use SnbtErrorKind::*;
        match self {
            ReachEndOfStream => write!(f, "reach end of stream"),
            InvalidEscapeSequence => write!(f, "invalid escape sequence"),
            EmptyKeyInCompound => write!(f, "empty key in compound"),
            ExpectColon => write!(f, "expect colon"),
            ExpectValue => write!(f, "expect value"),
            ExpectComma => write!(f, "expect comma"),
            WrongTypeInArray => write!(f, "wrong type in array"),
            DifferentTypesInList => write!(f, "different types in list"),
            LongString => write!(f, "long string"),
            TrailingData => write!(f, "extra data after end"),
            DepthLimitExceeded => write!(f, "depth limit exceeded"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct SnbtError {
    pub error_type: SnbtErrorKind,
    pub line: usize,
    pub column: usize,
}

impl SnbtError {
    pub fn new(error_type: SnbtErrorKind, line: usize, column: usize) -> Self {
        Self {
            error_type,
            line,
            column,
        }
    }
}

impl Display for SnbtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "@ {},{}: {}", self.line, self.column, self.error_type)
    }
}

impl Error for SnbtError {}

type Result<T> = std::result::Result<T, SnbtError>;

pub struct SnbtReader<'a> {
    line: usize,
    column: usize,
    index: usize,
    depth: usize,
    iter: Peekable<Chars<'a>>,
    pushed_back: Option<char>,
}

impl<'a> SnbtReader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            line: 1,
            column: 1,
            index: 0,
            depth: 0,
            iter: input.chars().peekable(),
            pushed_back: None,
        }
    }

    fn check_depth<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.depth >= MAX_DEPTH {
            Err(self.make_error(SnbtErrorKind::DepthLimitExceeded))
        } else {
            self.depth += 1;
            let res = f(self);
            self.depth -= 1;
            res
        }
    }

    fn make_error(&self, error_type: SnbtErrorKind) -> SnbtError {
        SnbtError::new(error_type, self.line, self.column)
    }

    fn peek(&mut self) -> Result<char> {
        if let Some(c) = self.pushed_back {
            Ok(c)
        } else {
            self.iter
                .peek()
                .copied()
                .ok_or_else(|| self.make_error(SnbtErrorKind::ReachEndOfStream))
        }
    }

    fn next(&mut self) {
        if self.pushed_back.is_some() {
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
            return Err(self.make_error(SnbtErrorKind::LongString));
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
                        return Err(self.make_error(SnbtErrorKind::InvalidEscapeSequence));
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
            return Err(self.make_error(SnbtErrorKind::LongString));
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
            if key.is_empty() {
                return Err(self.make_error(SnbtErrorKind::EmptyKeyInCompound));
            }
            if self.peek()? != ':' {
                return Err(self.make_error(SnbtErrorKind::ExpectColon));
            }
            self.next();
            self.skip_whitespace();
            let value = self.parse_element()?;
            self.skip_whitespace();
            if self.peek()? == ',' {
                self.next();
                self.skip_whitespace();
            } else if self.peek()? != '}' {
                return Err(self.make_error(SnbtErrorKind::ExpectComma));
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
                element_type = value.get_tag();
            } else if value.get_tag() != element_type {
                return Err(self.make_error(SnbtErrorKind::DifferentTypesInList));
            }
            if self.peek()? == ',' {
                self.next();
                self.skip_whitespace();
            } else if self.peek()? != ']' {
                return Err(self.make_error(SnbtErrorKind::ExpectComma));
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
            _ => return self.check_depth(|v| Ok(v.continue_parse_list()?.into())),
        };
        self.next();
        if self.peek()? != ';' {
            self.push_back(type_char);
            return self.check_depth(|v| Ok(v.continue_parse_list()?.into()));
        }
        self.next();
        self.skip_whitespace();
        let mut values = vec![];
        while self.peek()? != ']' {
            let value = self.parse_element()?;
            if value.get_tag() != etype {
                return Err(self.make_error(SnbtErrorKind::WrongTypeInArray));
            }
            values.push(value);
            self.skip_whitespace();
            if self.peek()? == ',' {
                self.next();
                self.skip_whitespace();
            } else if self.peek()? != ']' {
                return Err(self.make_error(SnbtErrorKind::ExpectComma));
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
            .ok_or_else(|| self.make_error(SnbtErrorKind::ExpectValue))?
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
            return Err(self.make_error(SnbtErrorKind::LongString));
        }
        Ok(Value::String(target))
    }

    /// Read the next element in the SNBT string.
    /// [`SnbtErrorKind::TrailingData`] cannot be returned because it is not
    /// considered to be an error.
    pub fn parse_element(&mut self) -> Result<Value> {
        self.skip_whitespace();
        match self.peek()? {
            '{' => self.check_depth(|v| Ok(v.parse_compound()?.into())),
            '[' => self.parse_list_like(),
            '"' | '\'' => self.read_quoted_string().map(|s| s.into()),
            _ => self.parse_primitive(),
        }
    }

    pub fn read(&mut self) -> Result<Value> {
        let value = self.parse_element()?;
        self.skip_whitespace();
        if self.peek().is_ok() {
            return Err(self.make_error(SnbtErrorKind::TrailingData));
        }
        Ok(value)
    }

    /// Get the number of bytes read.
    /// It's useful when you want to read a SNBT string from an command argument
    /// since there may be trailing data.
    pub fn bytes_read(&self) -> usize {
        self.index
    }
}
/// Parse a string in SNBT format into a `Value`.
/// Assert that the string has no trailing data.
/// SNBT is quite similar to JSON, but with some differences.
/// See [the wiki](https://minecraft.gamepedia.com/NBT_format#SNBT_format) for more information.
/// # Example
/// ```
/// use valence_nbt::snbt::SnbtReader;
/// use valence_nbt::Value;
/// let value = SnbtReader::from_snbt("1f").unwrap();
/// assert_eq!(value, Value::Float(1.0));
/// ```
pub fn from_snbt_string(snbt: &str) -> Result<Value> {
    SnbtReader::new(snbt).read()
}

pub struct SnbtWriter<'a> {
    output: &'a mut String,
}

impl<'a> SnbtWriter<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }

    fn write_string(&mut self, s: &str) {
        let mut need_quote = false;
        for c in s.chars() {
            if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '+' | '.') {
                need_quote = true;
                break;
            }
        }
        if need_quote {
            self.output.push('"');
            for c in s.chars() {
                match c {
                    '"' => self.output.push_str("\\\""),
                    '\\' => self.output.push_str("\\\\"),
                    _ => self.output.push(c),
                }
            }
            self.output.push('"');
        } else {
            self.output.push_str(s);
        }
    }

    fn write_primitive_array<'b>(
        &mut self,
        prefix: &str,
        iter: impl Iterator<Item = &'b (impl Into<Value> + 'b + Copy)>,
    ) {
        self.output.push('[');
        self.output.push_str(prefix);
        let mut first = true;
        for v in iter {
            if !first {
                self.output.push(',');
            }
            first = false;
            self.write_element(&(*v).into());
        }
        self.output.push(']');
    }

    fn write_primitive(&mut self, postfix: &str, value: impl ToString) {
        self.output.push_str(&value.to_string());
        self.output.push_str(postfix);
    }

    fn write_list(&mut self, list: &List) {
        macro_rules! variant_impl {
            ($v:expr, $handle:expr) => {{
                self.output.push('[');
                let mut first = true;
                for v in $v.iter() {
                    if !first {
                        self.output.push(',');
                    }
                    first = false;
                    $handle(v);
                }
                self.output.push(']');
            }};
        }
        match list {
            List::Byte(v) => variant_impl!(v, |v| self.write_primitive("b", v)),
            List::Short(v) => variant_impl!(v, |v| self.write_primitive("s", v)),
            List::Int(v) => variant_impl!(v, |v| self.write_primitive("", v)),
            List::Long(v) => variant_impl!(v, |v| self.write_primitive("l", v)),
            List::Float(v) => variant_impl!(v, |v| self.write_primitive("f", v)),
            List::Double(v) => variant_impl!(v, |v| self.write_primitive("d", v)),
            List::ByteArray(v) => {
                variant_impl!(v, |v: &Vec<i8>| self.write_primitive_array("B", v.iter()))
            }
            List::IntArray(v) => {
                variant_impl!(v, |v: &Vec<i32>| self.write_primitive_array("", v.iter()))
            }
            List::LongArray(v) => {
                variant_impl!(v, |v: &Vec<i64>| self.write_primitive_array("L", v.iter()))
            }
            List::String(v) => variant_impl!(v, |v| self.write_string(v)),
            List::List(v) => variant_impl!(v, |v| self.write_list(v)),
            List::Compound(v) => variant_impl!(v, |v| self.write_compound(v)),
            List::End => self.output.push_str("[]"),
        }
    }

    fn write_compound(&mut self, compound: &Compound) {
        self.output.push('{');
        let mut first = true;
        for (k, v) in compound.iter() {
            if !first {
                self.output.push(',');
            }
            first = false;
            self.write_string(k);
            self.output.push(':');
            self.write_element(v);
        }
        self.output.push('}');
    }

    /// Write a value to the output.
    pub fn write_element(&mut self, value: &Value) {
        use Value::*;
        match value {
            Byte(v) => self.write_primitive("b", v),
            Short(v) => self.write_primitive("s", v),
            Int(v) => self.write_primitive("", v),
            Long(v) => self.write_primitive("l", v),
            Float(v) => self.write_primitive("f", v),
            Double(v) => self.write_primitive("d", v),
            ByteArray(v) => self.write_primitive_array("B;", v.iter()),
            IntArray(v) => self.write_primitive_array("I;", v.iter()),
            LongArray(v) => self.write_primitive_array("L;", v.iter()),
            String(v) => self.write_string(v),
            List(v) => self.write_list(v),
            Compound(v) => self.write_compound(v),
        }
    }
}

/// Convert a value to a string in SNBT format.
pub fn to_snbt_string(value: &Value) -> String {
    let mut output = String::new();
    let mut writer = SnbtWriter::new(&mut output);
    writer.write_element(value);
    output
}

impl Display for SnbtWriter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output)
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
        let value = from_snbt_string(str).unwrap();
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
        let List::String(list) = cpd.get("empty").unwrap().as_list().unwrap() else { panic!() };
        assert_eq!(list[0], "Bibabo");
        assert_eq!(
            from_snbt_string("\"\\n\"").unwrap_err().error_type,
            SnbtErrorKind::InvalidEscapeSequence
        );
        assert_eq!(
            from_snbt_string("[L; 1]").unwrap_err().error_type,
            SnbtErrorKind::WrongTypeInArray
        );
        assert_eq!(
            from_snbt_string("[L; 1L, 2L, 3L").unwrap_err().error_type,
            SnbtErrorKind::ReachEndOfStream
        );
        assert_eq!(
            from_snbt_string("[L; 1L, 2L, 3L,]dewdwe")
                .unwrap_err()
                .error_type,
            SnbtErrorKind::TrailingData
        );
        assert_eq!(
            from_snbt_string("{ foo: }").unwrap_err().error_type,
            SnbtErrorKind::ExpectValue
        );
        assert_eq!(
            from_snbt_string("{ {}, }").unwrap_err().error_type,
            SnbtErrorKind::EmptyKeyInCompound
        );
        assert_eq!(
            from_snbt_string("{ foo 1 }").unwrap_err().error_type,
            SnbtErrorKind::ExpectColon
        );
        assert_eq!(
            from_snbt_string("{ foo: 1 bar: 2 }")
                .unwrap_err()
                .error_type,
            SnbtErrorKind::ExpectComma
        );
        assert_eq!(
            from_snbt_string("[{}, []]").unwrap_err().error_type,
            SnbtErrorKind::DifferentTypesInList
        );
        assert_eq!(
            from_snbt_string(&String::from_utf8(vec![b'e'; 32768]).unwrap())
                .unwrap_err()
                .error_type,
            SnbtErrorKind::LongString
        );
        assert_eq!(
            from_snbt_string(
                &String::from_utf8([[b'['; MAX_DEPTH + 1], [b']'; MAX_DEPTH + 1]].concat())
                    .unwrap()
            )
            .unwrap_err()
            .error_type,
            SnbtErrorKind::DepthLimitExceeded
        );
        #[cfg(feature = "preserve_order")]
        assert_eq!(
            to_snbt_string(&value),
            "{foo:1,bar:1d,baz:1f,\"hello'\":\"hello \
             world\",world:\"hello\\\"world\",1.5f:1.5d,3b:2f,bool:0b,more:{iarr:[I;1,2,3],larr:\
             [L;1l,2l,3l]},empty:[Bibabo]}"
        );
    }
}
