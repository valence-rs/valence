use std::str::CharIndices;

use crate::compound;
use crate::compound::Compound;
use crate::value::Value;

const EOF: char = '\0';

/// An implementation of an NBT path containing the source `String`, and a
/// vector of [NbtPathNode]
pub struct NbtPath {
    pub source: String,
    pub nodes: Vec<NbtPathNode>,
}

/// Representation of the various kinds of possible NBT path nodes.
///
/// All list index related nodes were condensed into enum member ListIndex, but
/// otherwise each enum member matches what is specified on [here](https://minecraft.fandom.com/wiki/NBT_path_format).
#[derive(Debug, PartialEq)]
pub enum NbtPathNode {
    RootCompoundTag {
        span: (usize, usize),
        compound: Compound,
    },
    NamedTag {
        span: (usize, usize),
        name: String,
    },
    NamedCompoundTag {
        span: (usize, usize),
        name: String,
        compound: Compound,
    },
    ListIndex {
        span: (usize, usize),
        name: String,
        indexes: Vec<IndexType>,
    },
}

/// Types of ways you can index into a list.
///
/// All: \[\]
///
/// Num: \[-1\], \[5\], \[21\]
///
/// Compound: \[{}\], \[{foo:"bar"}\], \["baz":53l\]
#[derive(Debug, PartialEq)]
pub enum IndexType {
    All,
    Num(i32),
    Compound(Compound),
}

impl TryFrom<String> for NbtPath {
    type Error = String;

    /// Convert a valid `String` into a [NbtPath].
    ///
    /// See [the format](https://minecraft.fandom.com/wiki/NBT_path_format) of an NBT Path.
    ///
    /// # Example
    ///
    /// ```
    /// use valence_nbt::path::{NbtPath, NbtPathNode};
    ///
    /// let nbt_path = NbtPath::try_from("{foo:1225l}.bar".to_string()).unwrap();
    ///
    /// assert_eq!(
    ///     nbt_path.nodes,
    ///     vec![
    ///         NbtPathNode::RootCompoundTag {
    ///             span: (0, 11),
    ///             compound: Some(compound!("foo" => 1225i64)),
    ///         },
    ///         NbtPathNode::NamedTag {
    ///             span: (12, 15),
    ///             name: "bar".to_string(),
    ///         }
    ///     ]
    /// );
    /// ```     

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut nbt_parser = NbtPathParser {
            string: &value,
            chars: value.char_indices(),
            index: 0,
        };
        let nodes: Vec<NbtPathNode> = nbt_parser.parse()?;
        Ok(Self {
            source: value,
            nodes,
        })
    }
}

struct NbtPathParser<'src> {
    string: &'src String,
    chars: CharIndices<'src>,
    index: usize,
}

impl NbtPathParser<'_> {
    fn skip_whitespace(&mut self) {
        while self.peek_char().is_ascii_whitespace() {
            self.bump();
        }
    }

    fn bump(&mut self) -> char {
        let next = self.chars.next();
        if next.is_none() {
            return EOF;
        }
        let next = next.unwrap();
        self.index = next.0;
        next.1
    }

    fn peek_char(&mut self) -> char {
        self.chars.clone().next().unwrap_or((0, EOF)).1
    }

    fn peek_char_index(&mut self) -> usize {
        self.chars
            .clone()
            .next()
            .unwrap_or((self.string.len(), EOF))
            .0
    }

    fn parse(&mut self) -> Result<Vec<NbtPathNode>, String> {
        let mut nodes = Vec::new();

        match self.peek_char() {
            '{' => {
                let span_start = self.peek_char_index();
                let root_compound = self.compound()?;
                let span_end = self.peek_char_index();
                nodes.push(NbtPathNode::RootCompoundTag {
                    span: (span_start, span_end),
                    compound: root_compound,
                });

                let peek = self.peek_char();
                // This is fine, just means that it's only a root node
                if peek == EOF {
                    return Ok(nodes);
                // This is not fine.
                } else if peek != '.' {
                    return Err(format!(
                        "Expected end or '.' for NBT path continuation, but found {}. (Index: {})",
                        peek,
                        self.peek_char_index()
                    ));
                } else {
                    self.bump();
                }
            }
            EOF => return Ok(nodes),
            _ => (),
        }
        loop {
            let span_start = self.peek_char_index();
            let name = match self.peek_char() {
                EOF => break,
                '"' => self.quoted_string(),
                _ => self.unquoted_name(),
            }?;
            let node = match self.peek_char() {
                '{' => {
                    let compound = self.compound()?;
                    NbtPathNode::NamedCompoundTag {
                        span: (span_start, self.peek_char_index()),
                        name,
                        compound,
                    }
                }
                '[' => {
                    let indexes = self.list_indexings()?;
                    let span_end = self.peek_char_index();
                    NbtPathNode::ListIndex {
                        span: (span_start, span_end),
                        name,
                        indexes,
                    }
                }
                _ => NbtPathNode::NamedTag {
                    span: (span_start, self.peek_char_index()),
                    name,
                },
            };

            nodes.push(node);
            if matches!(self.peek_char(), EOF | '.') {
                self.bump();
            }
        }
        Ok(nodes)
    }

    fn list_indexings(&mut self) -> Result<Vec<IndexType>, String> {
        let mut indexes: Vec<IndexType> = vec![];
        while self.peek_char() == '[' {
            self.bump();
            match self.peek_char() {
                ']' => indexes.push(IndexType::All),
                '{' => indexes.push(IndexType::Compound(self.compound()?)),
                '-' | '0'..='9' => indexes.push(IndexType::Num(self.index_num()?)),
                ch => {
                    return Err(format!(
                        "Expected closing square bracket, compound start, or number, but found \
                         {}. (Index: {})",
                        ch,
                        self.peek_char_index()
                    ))
                }
            }
            let next = self.bump();
            if next != ']' {
                return Err(format!(
                    "Expected closing square bracket, but found {}. (Index: {})",
                    next, self.index
                ));
            }
        }
        Ok(indexes)
    }

    fn index_num(&mut self) -> Result<i32, String> {
        if self.peek_char() == ']' {
            return Err(format!(
                "Expected number, found closing square bracket. (Index: {})",
                self.peek_char_index()
            ));
        }

        let start = self.peek_char_index();
        if self.peek_char() == '-' {
            self.bump();
        }

        while let '0'..='9' = self.peek_char() {
            self.bump();
        }

        let end = self.peek_char_index();
        let num_string = &self.string[start..end];
        parse_num::<i32>(num_string, self.index)
    }

    fn compound(&mut self) -> Result<Compound, String> {
        // bump past the starting curly bracket
        self.bump();
        self.skip_whitespace();
        // if there's already a closing bracket, it's just an empty compound
        if self.peek_char() == '}' {
            self.bump();
            return Ok(compound!());
        }
        // get the label
        let label: String = if self.peek_char() == '"' {
            self.quoted_string()
        } else {
            self.unquoted_name()
        }?;
        self.skip_whitespace();
        if self.peek_char() != ':' {
            return Err(format!(
                "Expected symbol ':' in compound, but found {}. (Index: {})",
                self.peek_char(),
                self.peek_char_index()
            ));
        }
        self.bump();
        self.skip_whitespace();
        // get the value
        let value: Value = self.value()?;
        self.skip_whitespace();
        // bump past the ending closing curly bracket
        if self.peek_char() != '}' {
            return Err(format!(
                "Expected symbol '}}' to end compound, but found {}. (Index: {})",
                self.peek_char(),
                self.peek_char_index()
            ));
        }
        self.bump();

        let compound = compound!(label => value);
        Ok(compound)
    }

    fn value(&mut self) -> Result<Value, String> {
        loop {
            match self.peek_char() {
                '"' => {
                    let value = self.quoted_string()?;
                    return Ok(Value::String(value));
                }
                '0'..='9' | '-' => {
                    return self.number();
                }

                _ => todo!(),
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let mut is_decimal = false;
        let num_start = self.peek_char_index();
        self.bump();
        while matches!(self.peek_char(), '0'..='9' | '.') {
            if self.peek_char() == '.' && !is_decimal {
                is_decimal = true;
            } else if self.peek_char() == '.' && is_decimal {
                return Err(format!(
                    "Too many '.' in decimal number. There was already one, but another was \
                     found. (Index: {})",
                    self.peek_char_index()
                ));
            }
            self.bump();
        }
        let num_end = self.peek_char_index();
        let num_string = &self.string[num_start..num_end];
        let type_indicator = self.peek_char();

        if is_decimal && !matches!(type_indicator, 'f' | 'F') {
            let num = parse_num::<f64>(num_string, self.index)?;
            return Ok(Value::Double(num));
        }

        match type_indicator {
            'b' | 'B' => {
                self.bump();
                let num = parse_num::<i8>(num_string, self.index)?;
                Ok(Value::Byte(num))
            }
            's' | 'S' => {
                self.bump();
                let num = parse_num::<i16>(num_string, self.index)?;
                Ok(Value::Short(num))
            }
            'l' | 'L' => {
                self.bump();
                let num = parse_num::<i64>(num_string, self.index)?;
                Ok(Value::Long(num))
            }
            'f' | 'F' => {
                self.bump();
                let num = parse_num::<f32>(num_string, self.index)?;
                Ok(Value::Float(num))
            }
            _ => {
                let num = parse_num::<i32>(num_string, self.index)?;
                Ok(Value::Int(num))
            }
        }
    }

    fn quoted_string(&mut self) -> Result<String, String> {
        self.bump();
        let mut name = String::new();
        loop {
            match self.peek_char() {
                // maybe escape sequence
                '\\' => {
                    self.bump();
                    match self.peek_char() {
                        '\\' | '"' => name.push(self.bump()),
                        'n' => {
                            self.bump();
                            name.push('\n');
                        }
                        't' => {
                            self.bump();
                            name.push('\t');
                        }
                        _ => name.push('\\'),
                    }
                }
                EOF => {
                    return Err(format!(
                        "Expected a quoted named tag terminated by a second \", but reached EOF. \
                         (Index: {})",
                        self.peek_char_index()
                    ));
                }
                '"' => break,
                _ => {
                    name.push(self.bump());
                }
            }
        }
        self.bump();
        Ok(name)
    }

    fn unquoted_name(&mut self) -> Result<String, String> {
        let span_start = self.peek_char_index();
        // Consume the whole name
        while let 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' = self.peek_char() {
            self.bump();
        }
        let span_end = self.peek_char_index();
        if span_start == span_end {
            return Err(format!(
                "Expected named tag, but found {}. (Index: {})",
                self.peek_char(),
                self.peek_char_index()
            ));
        }
        let name = self.string[span_start..span_end].to_owned();
        Ok(name)
    }
}

fn parse_num<T: std::str::FromStr>(string: &str, index: usize) -> Result<T, String> {
    let parsed = string.parse::<T>();
    match parsed {
        Err(_) => Err(format!(
            "Attempted to parse {} to a {}, but it failed. (Index: {})",
            string,
            std::any::type_name::<T>(),
            index
        )),
        Ok(value) => Ok(value),
    }
}

#[cfg(test)]
mod nbt_path_tests {
    use crate::compound;
    use crate::path::{IndexType, NbtPath, NbtPathNode};

    #[test]
    fn path1() {
        let name = NbtPath::try_from("{foo:\"bar\"}".to_string()).unwrap();
        assert_eq!(
            name.nodes[0],
            NbtPathNode::RootCompoundTag {
                span: (0, 11),
                compound: compound!("foo" => "bar"),
            }
        );
    }

    #[test]
    fn path2() {
        let name = NbtPath::try_from("{foo:1225l}.bar".to_string()).unwrap();
        assert_eq!(
            name.nodes,
            vec![
                NbtPathNode::RootCompoundTag {
                    span: (0, 11),
                    compound: compound!("foo" => 1225i64),
                },
                NbtPathNode::NamedTag {
                    span: (12, 15),
                    name: "bar".to_string(),
                }
            ]
        );
    }

    #[test]
    fn path3() {
        let path = NbtPath::try_from("foo.bar.baz".to_string()).unwrap();
        assert_eq!(
            path.nodes,
            vec![
                NbtPathNode::NamedTag {
                    span: (0, 3),
                    name: "foo".to_string()
                },
                NbtPathNode::NamedTag {
                    span: (4, 7),
                    name: "bar".to_string()
                },
                NbtPathNode::NamedTag {
                    span: (8, 11),
                    name: "baz".to_string()
                },
            ]
        );
    }

    #[test]
    fn path4() {
        let nbt_path = NbtPath::try_from("{foo:-432.40f}.bar{baz:5b}.qux".to_string()).unwrap();
        assert_eq!(
            nbt_path.nodes,
            vec![
                NbtPathNode::RootCompoundTag {
                    span: (0, 14),
                    compound: compound!("foo" => -432.40f32),
                },
                NbtPathNode::NamedCompoundTag {
                    span: (15, 26),
                    name: "bar".to_string(),
                    compound: compound!("baz" => 5i8),
                },
                NbtPathNode::NamedTag {
                    span: (27, 30),
                    name: "qux".to_string(),
                }
            ]
        );
    }

    #[test]
    fn path5() {
        let nbt_path = NbtPath::try_from("\"aa\\n\"".to_string()).unwrap();
        assert_eq!(
            nbt_path.nodes,
            vec![NbtPathNode::NamedTag {
                span: (0, 6),
                name: "aa\n".to_string(),
            },]
        );
    }

    #[test]
    fn path6() {
        let nbt_path =
            NbtPath::try_from("foo[-1][{ \"guacamole\" :920S   }][]".to_string()).unwrap();
        assert_eq!(
            nbt_path.nodes,
            vec![NbtPathNode::ListIndex {
                span: (0, 34),
                name: "foo".to_string(),
                indexes: vec![
                    IndexType::Num(-1),
                    IndexType::Compound(compound!("guacamole" => 920i16)),
                    IndexType::All,
                ],
            },]
        );
    }

    #[test]
    fn path7() {
        let nbt_path =
            NbtPath::try_from("{ }.all.the{}.different[-1].parts[{are: \"here\"}][]".to_string())
                .unwrap();
        assert_eq!(
            nbt_path.nodes,
            vec![
                NbtPathNode::RootCompoundTag {
                    span: (0, 3),
                    compound: compound!(),
                },
                NbtPathNode::NamedTag {
                    span: (4, 7),
                    name: "all".to_string()
                },
                NbtPathNode::NamedCompoundTag {
                    span: (8, 13),
                    name: "the".to_string(),
                    compound: compound!(),
                },
                NbtPathNode::ListIndex {
                    span: (14, 27),
                    name: "different".to_string(),
                    indexes: vec![IndexType::Num(-1)],
                },
                NbtPathNode::ListIndex {
                    span: (28, 50),
                    name: "parts".to_string(),
                    indexes: vec![
                        IndexType::Compound(compound!("are" => "here")),
                        IndexType::All
                    ],
                },
            ]
        );
    }

    #[test]
    #[should_panic]
    fn path8() {
        NbtPath::try_from("foo.bar{]".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn path9() {
        NbtPath::try_from("[]".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn path10() {
        NbtPath::try_from("{}.{}".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn path11() {
        NbtPath::try_from("{:5}".to_string()).unwrap();
    }
}
