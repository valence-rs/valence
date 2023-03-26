use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use byteorder::WriteBytesExt;

use crate::ident::Ident;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct CommandTreeS2c<'a> {
    pub commands: Vec<Node<'a>>,
    pub root_index: VarInt,
}

#[derive(Clone, Debug)]
pub struct Node<'a> {
    pub children: Vec<VarInt>,
    pub data: NodeData<'a>,
    pub executable: bool,
    pub redirect_node: Option<VarInt>,
}

#[derive(Clone, Debug)]
pub enum NodeData<'a> {
    Root,
    Literal {
        name: &'a str,
    },
    Argument {
        name: &'a str,
        parser: Parser<'a>,
        suggestion: Option<Suggestion>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Suggestion {
    AskServer,
    AllRecipes,
    AvailableSounds,
    AvailableBiomes,
    SummonableEntities,
}

#[derive(Clone, Debug)]
pub enum Parser<'a> {
    Bool,
    Float { min: Option<f32>, max: Option<f32> },
    Double { min: Option<f64>, max: Option<f64> },
    Integer { min: Option<i32>, max: Option<i32> },
    Long { min: Option<i64>, max: Option<i64> },
    String(StringArg),
    Entity { single: bool, only_players: bool },
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    Component,
    Message,
    NbtCompoundTag,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder { allow_multiple: bool },
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    Dimension,
    GameMode,
    Time,
    ResourceOrTag { registry: Ident<Cow<'a, str>> },
    ResourceOrTagKey { registry: Ident<Cow<'a, str>> },
    Resource { registry: Ident<Cow<'a, str>> },
    ResourceKey { registry: Ident<Cow<'a, str>> },
    TemplateMirror,
    TemplateRotation,
    Uuid,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum StringArg {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

impl Encode for Node<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let node_type = match &self.data {
            NodeData::Root => 0,
            NodeData::Literal { .. } => 1,
            NodeData::Argument { .. } => 2,
        };

        let has_suggestion = matches!(
            &self.data,
            NodeData::Argument {
                suggestion: Some(_),
                ..
            }
        );

        let flags: u8 = node_type
            | (self.executable as u8 * 0x04)
            | (self.redirect_node.is_some() as u8 * 0x08)
            | (has_suggestion as u8 * 0x10);

        w.write_u8(flags)?;

        self.children.encode(&mut w)?;

        if let Some(redirect_node) = self.redirect_node {
            redirect_node.encode(&mut w)?;
        }

        match &self.data {
            NodeData::Root => {}
            NodeData::Literal { name } => {
                name.encode(&mut w)?;
            }
            NodeData::Argument {
                name,
                parser,
                suggestion,
            } => {
                name.encode(&mut w)?;
                parser.encode(&mut w)?;

                if let Some(suggestion) = suggestion {
                    match suggestion {
                        Suggestion::AskServer => "ask_server",
                        Suggestion::AllRecipes => "all_recipes",
                        Suggestion::AvailableSounds => "available_sounds",
                        Suggestion::AvailableBiomes => "available_biomes",
                        Suggestion::SummonableEntities => "summonable_entities",
                    }
                    .encode(&mut w)?;
                }
            }
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for Node<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let flags = u8::decode(r)?;

        let children = Vec::decode(r)?;

        let redirect_node = if flags & 0x08 != 0 {
            Some(VarInt::decode(r)?)
        } else {
            None
        };

        let node_data = match flags & 0x3 {
            0 => NodeData::Root,
            1 => NodeData::Literal {
                name: <&str>::decode(r)?,
            },
            2 => NodeData::Argument {
                name: <&str>::decode(r)?,
                parser: Parser::decode(r)?,
                suggestion: if flags & 0x10 != 0 {
                    Some(match Ident::<Cow<str>>::decode(r)?.as_str() {
                        "minecraft:ask_server" => Suggestion::AskServer,
                        "minecraft:all_recipes" => Suggestion::AllRecipes,
                        "minecraft:available_sounds" => Suggestion::AvailableSounds,
                        "minecraft:available_biomes" => Suggestion::AvailableBiomes,
                        "minecraft:summonable_entities" => Suggestion::SummonableEntities,
                        other => bail!("unknown command suggestion type of \"{other}\""),
                    })
                } else {
                    None
                },
            },
            n => bail!("invalid node type of {n}"),
        };

        Ok(Self {
            children,
            data: node_data,
            executable: flags & 0x04 != 0,
            redirect_node,
        })
    }
}

impl Encode for Parser<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            Parser::Bool => 0u8.encode(&mut w)?,
            Parser::Float { min, max } => {
                1u8.encode(&mut w)?;

                (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                if let Some(min) = min {
                    min.encode(&mut w)?;
                }

                if let Some(max) = max {
                    max.encode(&mut w)?;
                }
            }
            Parser::Double { min, max } => {
                2u8.encode(&mut w)?;

                (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                if let Some(min) = min {
                    min.encode(&mut w)?;
                }

                if let Some(max) = max {
                    max.encode(&mut w)?;
                }
            }
            Parser::Integer { min, max } => {
                3u8.encode(&mut w)?;

                (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                if let Some(min) = min {
                    min.encode(&mut w)?;
                }

                if let Some(max) = max {
                    max.encode(&mut w)?;
                }
            }
            Parser::Long { min, max } => {
                4u8.encode(&mut w)?;

                (min.is_some() as u8 | (max.is_some() as u8 * 0x2)).encode(&mut w)?;

                if let Some(min) = min {
                    min.encode(&mut w)?;
                }

                if let Some(max) = max {
                    max.encode(&mut w)?;
                }
            }
            Parser::String(arg) => {
                5u8.encode(&mut w)?;
                arg.encode(&mut w)?;
            }
            Parser::Entity {
                single,
                only_players,
            } => {
                6u8.encode(&mut w)?;
                (*single as u8 | (*only_players as u8 * 0x2)).encode(&mut w)?;
            }
            Parser::GameProfile => 7u8.encode(&mut w)?,
            Parser::BlockPos => 8u8.encode(&mut w)?,
            Parser::ColumnPos => 9u8.encode(&mut w)?,
            Parser::Vec3 => 10u8.encode(&mut w)?,
            Parser::Vec2 => 11u8.encode(&mut w)?,
            Parser::BlockState => 12u8.encode(&mut w)?,
            Parser::BlockPredicate => 13u8.encode(&mut w)?,
            Parser::ItemStack => 14u8.encode(&mut w)?,
            Parser::ItemPredicate => 15u8.encode(&mut w)?,
            Parser::Color => 16u8.encode(&mut w)?,
            Parser::Component => 17u8.encode(&mut w)?,
            Parser::Message => 18u8.encode(&mut w)?,
            Parser::NbtCompoundTag => 19u8.encode(&mut w)?,
            Parser::NbtTag => 20u8.encode(&mut w)?,
            Parser::NbtPath => 21u8.encode(&mut w)?,
            Parser::Objective => 22u8.encode(&mut w)?,
            Parser::ObjectiveCriteria => 23u8.encode(&mut w)?,
            Parser::Operation => 24u8.encode(&mut w)?,
            Parser::Particle => 25u8.encode(&mut w)?,
            Parser::Angle => 26u8.encode(&mut w)?,
            Parser::Rotation => 27u8.encode(&mut w)?,
            Parser::ScoreboardSlot => 28u8.encode(&mut w)?,
            Parser::ScoreHolder { allow_multiple } => {
                29u8.encode(&mut w)?;
                allow_multiple.encode(&mut w)?;
            }
            Parser::Swizzle => 30u8.encode(&mut w)?,
            Parser::Team => 31u8.encode(&mut w)?,
            Parser::ItemSlot => 32u8.encode(&mut w)?,
            Parser::ResourceLocation => 33u8.encode(&mut w)?,
            Parser::Function => 34u8.encode(&mut w)?,
            Parser::EntityAnchor => 35u8.encode(&mut w)?,
            Parser::IntRange => 36u8.encode(&mut w)?,
            Parser::FloatRange => 37u8.encode(&mut w)?,
            Parser::Dimension => 38u8.encode(&mut w)?,
            Parser::GameMode => 39u8.encode(&mut w)?,
            Parser::Time => 40u8.encode(&mut w)?,
            Parser::ResourceOrTag { registry } => {
                41u8.encode(&mut w)?;
                registry.encode(&mut w)?;
            }
            Parser::ResourceOrTagKey { registry } => {
                42u8.encode(&mut w)?;
                registry.encode(&mut w)?;
            }
            Parser::Resource { registry } => {
                43u8.encode(&mut w)?;
                registry.encode(&mut w)?;
            }
            Parser::ResourceKey { registry } => {
                44u8.encode(&mut w)?;
                registry.encode(&mut w)?;
            }
            Parser::TemplateMirror => 45u8.encode(&mut w)?,
            Parser::TemplateRotation => 46u8.encode(&mut w)?,
            Parser::Uuid => 47u8.encode(&mut w)?,
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for Parser<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        fn decode_min_max<'a, T: Decode<'a>>(
            r: &mut &'a [u8],
        ) -> anyhow::Result<(Option<T>, Option<T>)> {
            let flags = u8::decode(r)?;

            let min = if flags & 0x1 != 0 {
                Some(T::decode(r)?)
            } else {
                None
            };

            let max = if flags & 0x2 != 0 {
                Some(T::decode(r)?)
            } else {
                None
            };

            Ok((min, max))
        }

        Ok(match u8::decode(r)? {
            0 => Self::Bool,
            1 => {
                let (min, max) = decode_min_max(r)?;
                Self::Float { min, max }
            }
            2 => {
                let (min, max) = decode_min_max(r)?;
                Self::Double { min, max }
            }
            3 => {
                let (min, max) = decode_min_max(r)?;
                Self::Integer { min, max }
            }
            4 => {
                let (min, max) = decode_min_max(r)?;
                Self::Long { min, max }
            }
            5 => Self::String(StringArg::decode(r)?),
            6 => {
                let flags = u8::decode(r)?;
                Self::Entity {
                    single: flags & 0x1 != 0,
                    only_players: flags & 0x2 != 0,
                }
            }
            7 => Self::GameProfile,
            8 => Self::BlockPos,
            9 => Self::ColumnPos,
            10 => Self::Vec3,
            11 => Self::Vec2,
            12 => Self::BlockState,
            13 => Self::BlockPredicate,
            14 => Self::ItemStack,
            15 => Self::ItemPredicate,
            16 => Self::Color,
            17 => Self::Component,
            18 => Self::Message,
            19 => Self::NbtCompoundTag,
            20 => Self::NbtTag,
            21 => Self::NbtPath,
            22 => Self::Objective,
            23 => Self::ObjectiveCriteria,
            24 => Self::Operation,
            25 => Self::Particle,
            26 => Self::Angle,
            27 => Self::Rotation,
            28 => Self::ScoreboardSlot,
            29 => Self::ScoreHolder {
                allow_multiple: bool::decode(r)?,
            },
            30 => Self::Swizzle,
            31 => Self::Team,
            32 => Self::ItemSlot,
            33 => Self::ResourceLocation,
            34 => Self::Function,
            35 => Self::EntityAnchor,
            36 => Self::IntRange,
            37 => Self::FloatRange,
            38 => Self::Dimension,
            39 => Self::GameMode,
            40 => Self::Time,
            41 => Self::ResourceOrTag {
                registry: Ident::decode(r)?,
            },
            42 => Self::ResourceOrTagKey {
                registry: Ident::decode(r)?,
            },
            43 => Self::Resource {
                registry: Ident::decode(r)?,
            },
            44 => Self::ResourceKey {
                registry: Ident::decode(r)?,
            },
            45 => Self::TemplateMirror,
            46 => Self::TemplateRotation,
            47 => Self::Uuid,
            n => bail!("unknown command parser ID of {n}"),
        })
    }
}
