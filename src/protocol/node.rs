use std::io::{Read, Write};

use anyhow::{bail, Context};
use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::ident::Ident;
use crate::protocol::{BoundedString, Decode, Encode, VarInt};

#[derive(Clone, Debug)]
pub struct Node {
    children: Vec<VarInt>,
    data: NodeData,
    is_executable: bool,
    redirect_node: Option<VarInt>,
}

impl Encode for Node {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let enum_id = match self.data {
            NodeData::Root => 0,
            NodeData::Literal(_) => 1,
            NodeData::Argument(_) => 2,
        };

        let flags = enum_id
            & (self.is_executable as u8 * 0x04)
            & (self.redirect_node.is_some() as u8 * 0x08)
            & (if let NodeData::Argument(argument) = &self.data {
                argument.suggestions_type.is_some()
            } else { false } as u8 * 0x04);

        w.write_u8(flags)?;
        self.children.encode(w)?;

        if let Some(redirect_node) = self.redirect_node {
            redirect_node.encode(w)?;
        }

        match &self.data {
            NodeData::Root => {}
            NodeData::Literal(literal) => {
                literal.name.encode(w)?
            }
            NodeData::Argument(argument) => {
                argument.name.encode(w)?;
                argument.parser.encode(w)?;
                if let Some(suggestions_type) = &argument.suggestions_type {
                    suggestions_type.encode(w)?;
                }
            }
        }

        Ok(())
    }
}

impl Decode for Node {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;

        let is_executable = flags & 0x04 != 0;
        let redirect_node = if flags & 0x08 != 0 {
            Decode::decode(r)?
        } else {
            None
        };

        let children = Decode::decode(r)?;

        let enum_id = flags & 0x03;
        let data = match enum_id {
            0 => NodeData::Root,
            1 => NodeData::Literal(Literal {
                name: Decode::decode(r)?,
            }),
            2 => NodeData::Argument(Argument {
                name: Decode::decode(r)?,
                parser: Decode::decode(r)?,
                suggestions_type: if flags & 0x10 != 0 {
                    Decode::decode(r)?
                } else {
                    None
                },
            }),
            _ => bail!("Invalid NodeData variant")
        };

        Ok(Node {
            children,
            data,
            is_executable,
            redirect_node,
        })
    }
}

#[derive(Clone, Debug)]
pub enum NodeData {
    Root,
    Literal(Literal),
    Argument(Argument),
}

#[derive(Clone, Debug)]
pub struct Literal {
    name: BoundedString<0, 32767>,
}

#[derive(Clone, Debug)]
pub struct Argument {
    name: BoundedString<0, 32767>,
    parser: Parser,
    suggestions_type: Option<Ident>,
}

def_enum! {
    Parser: VarInt {
        BrigadierBool: bool = 0,
        BrigadierFloat: BrigadierFloat = 1,
        BrigadierInteger: BrigadierInteger = 2,
        BrigadierLong: BrigadierLong = 3,
        //TODO
    }
}

#[derive(Clone, Debug)]
pub struct BrigadierFloat {
    min: Option<f32>,
    max: Option<f32>,
}

impl Encode for BrigadierFloat {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let flags = (self.min.is_some() as u8) << 0 & (self.max.is_some() as u8) << 1;
        w.write_u8(flags)?;
        if let Some(min) = self.min {
            min.encode(w)?;
        }
        if let Some(max) = self.max {
            max.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for BrigadierFloat {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;
        let min = if flags & 0x01 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        let max = if flags & 0x02 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        Ok(Self { min, max })
    }
}

#[derive(Clone, Debug)]
pub struct BrigadierInteger {
    min: Option<i32>,
    max: Option<i32>,
}

impl Encode for BrigadierInteger {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let flags = (self.min.is_some() as u8) << 0 & (self.max.is_some() as u8) << 1;
        w.write_u8(flags)?;
        if let Some(min) = self.min {
            min.encode(w)?;
        }
        if let Some(max) = self.max {
            max.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for BrigadierInteger {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;
        let min = if flags & 0x01 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        let max = if flags & 0x02 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        Ok(Self { min, max })
    }
}

#[derive(Clone, Debug)]
pub struct BrigadierLong {
    min: Option<i64>,
    max: Option<i64>,
}

impl Encode for BrigadierLong {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let flags = (self.min.is_some() as u8) << 0 & (self.max.is_some() as u8) << 1;
        w.write_u8(flags)?;
        if let Some(min) = self.min {
            min.encode(w)?;
        }
        if let Some(max) = self.max {
            max.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for BrigadierLong {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;
        let min = if flags & 0x01 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        let max = if flags & 0x02 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        Ok(Self { min, max })
    }
}