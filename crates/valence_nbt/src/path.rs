use std::str::CharIndices;

use crate::compound;
use crate::compound::Compound;
use crate::value::Value;

const EOF: char = '\0';

pub struct NbtPath {
    string: String,
    nodes: Vec<NbtPathNode>,
}

#[derive(Debug, PartialEq)]
pub enum NbtPathNode {
    RootCompoundTag {
        span: (usize, usize),
        compound: Option<Compound>,
    },
    NamedTag {
        span: (usize, usize),
        name: String,
    },
    NamedCompoundTag {
        span: (usize, usize),
        name: String,
        compound: Option<Compound>,
    },
    ElementOfList {
        span: (usize, usize),
        list_name: String,
        index: i32,
    },
    AllElementsOfList {
        span: (usize, usize),
        list_name: String,
    },
    CompoundElementsOfList {
        span: (usize, usize),
        list_name: String,
        compound: Option<Compound>,
    },
    ElementsOfSubList {
        span: (usize, usize),
        list_name: String,
        indexes: Vec<Option<i32>>,
        compound: Option<Compound>,
    },
}

impl TryFrom<String> for NbtPath {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut nbt_parser = NbtPathParser {
            string: &value,
            chars: value.char_indices(),
            index: 0,
        };
        let nodes: Vec<NbtPathNode> = nbt_parser.parse()?;
        Ok(Self {
            string: value,
            nodes,
        })
    }
}

struct NbtPathParser<'a> {
    string: &'a String,
    chars: CharIndices<'a>,
    index: usize,
}

impl NbtPathParser<'_> {
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
                let root_compound = self.compound()?;
                let end = self.peek_char_index();
                nodes.push(NbtPathNode::RootCompoundTag {
                    span: (root_compound.0, end),
                    compound: root_compound.1,
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
            let (span_start, name) = match self.peek_char() {
                EOF => break,
                '"' => self.quoted_name(),
                _ => self.unquoted_name(),
            }?;
            if matches!(self.peek_char(), EOF | '.') {
                nodes.push(NbtPathNode::NamedTag {
                    span: (span_start, self.peek_char_index()),
                    name,
                });
                self.bump();
                continue;
            }
            let node = match self.peek_char() {
                '{' => {
                    let compound = self.compound()?.1;
                    NbtPathNode::NamedCompoundTag {
                        span: (span_start, self.peek_char_index()),
                        name,
                        compound,
                    }
                }
                _ => todo!(),
            };
            nodes.push(node);
        }
        Ok(nodes)
    }
    fn compound(&mut self) -> Result<(usize, Option<Compound>), String> {
        // bump past the starting curly bracket
        self.bump();

        let span_start = self.index;

        // if there's already a closing bracket, it's just an empty compound
        if self.peek_char() == '}' {
            self.bump();
            return Ok((span_start, None));
        }
        // get the label
        let label: String = self.unquoted_name()?.1;
        if self.peek_char() != ':' {
            return Err(format!(
                "Expected symbol ':' in compound, but found {}. (Index: {})",
                self.peek_char(),
                self.peek_char_index()
            ));
        }
        self.bump();
        // get the value
        let value: Value = self.value()?.1;

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
        Ok((span_start, Some(compound)))
    }
    fn value(&mut self) -> Result<(usize, Value), String> {
        let span_start = self.peek_char_index();
        loop {
            match self.peek_char() {
                '"' => {
                    let value = self.quoted_name()?.1;
                    return Ok((span_start, Value::String(value)));
                }
                '0'..='9' | '-' => {
                    return self.number();
                }

                _ => todo!(),
            }
        }
    }
    fn number(&mut self) -> Result<(usize, Value), String> {
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
            let num = parse_or_error::<f64>(num_string, self.index)?;
            return Ok((num_start, Value::Double(num)));
        }

        match type_indicator {
            'b' | 'B' => {
                self.bump();
                let num = parse_or_error::<i8>(num_string, self.index)?;
                return Ok((num_start, Value::Byte(num)));
            }
            's' | 'S' => {
                self.bump();
                let num = parse_or_error::<i16>(num_string, self.index)?;
                return Ok((num_start, Value::Short(num)));
            }
            'l' | 'L' => {
                self.bump();
                let num = parse_or_error::<i64>(num_string, self.index)?;
                return Ok((num_start, Value::Long(num)));
            }
            'f' | 'F' => {
                self.bump();
                let num = parse_or_error::<f32>(num_string, self.index)?;
                return Ok((num_start, Value::Float(num)));
            }

            _ => {
                let num = parse_or_error::<i32>(num_string, self.index)?;
                return Ok((num_start, Value::Int(num)));
            }
        }
    }
    fn quoted_name(&mut self) -> Result<(usize, String), String> {
        self.bump();
        let span_start = self.index;
        loop {
            println!("next: {}", self.peek_char_index());
            match self.peek_char() {
                EOF => {
                    return Err(format!(
                        "Expected a quoted named tag terminated by a second \", but reached EOF. \
                         (Index: {})",
                        self.peek_char_index()
                    ));
                }
                '"' => break,
                _ => {
                    self.bump();
                    println!("index {}", self.index);
                }
            }
        }
        self.bump();
        let span_end = self.peek_char_index();
        let name = self.string[(span_start + 1)..(span_end - 1)].to_owned();
        Ok((span_start, name))
    }
    fn unquoted_name(&mut self) -> Result<(usize, String), String> {
        let span_start = self.peek_char_index();
        loop {
            match self.peek_char() {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => self.bump(),
                _ => break,
            };
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
        Ok((span_start, name))
    }
}

fn parse_or_error<T: std::str::FromStr>(string: &str, index: usize) -> Result<T, String> {
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
    use crate::path::{NbtPath, NbtPathNode};

    #[test]
    fn named_tag() {
        let name = NbtPath::try_from("hello".to_string()).unwrap();
        assert_eq!(
            name.nodes[0],
            NbtPathNode::NamedTag {
                span: (0, 5),
                name: "hello".to_string()
            }
        );
    }
    #[test]
    fn escaped_named_tag() {
        let name = NbtPath::try_from("\"@(#hello >\"".to_string()).unwrap();
        assert_eq!(
            name.nodes[0],
            NbtPathNode::NamedTag {
                span: (0, 12),
                name: "@(#hello >".to_string()
            }
        );
    }
    #[test]
    fn name_then_name() {
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
    fn empty_root_then_named_tag() {
        let nbt_path = NbtPath::try_from("{help:-432.40f}.foo{bar:5b}.baz".to_string()).unwrap();
        assert_eq!(
            nbt_path.nodes,
            vec![
                NbtPathNode::RootCompoundTag {
                    span: (0, 15),
                    compound: Some(compound!("help" => -432.40f32)),
                },
                NbtPathNode::NamedCompoundTag {
                    span: (16, 27),
                    name: "foo".to_string(),
                    compound: Some(compound!("bar" => 5i8)),
                },
                NbtPathNode::NamedTag {
                    span: (28, 31),
                    name: "baz".to_string(),
                }
            ]
        );
    }
}
