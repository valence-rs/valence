use std::borrow::Cow;
use std::mem::MaybeUninit;

use glam::IVec3;
use parser::StrReader;
use valence_core::block_pos::BlockPos;
use valence_core::packet::s2c::play::command_tree as pkt;
use valence_core::text::Text;

pub mod parser;

pub const EXPECTED_ARGUMENTS_SEPARATOR: &str = "command.expected.separator";
pub const ARGUMENT_POS_MIXED: &str = "argument.pos.mixed";

#[derive(Debug)]
pub enum ArgumentNodeParseError {
    User(Text),
    Internal(anyhow::Error),
}

impl From<Text> for ArgumentNodeParseError {
    fn from(value: Text) -> Self {
        Self::User(value)
    }
}

impl From<anyhow::Error> for ArgumentNodeParseError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

pub trait ArgumentNode<'a>: ParsableObject<'a> {
    fn brigadier_parser<'d>(data: &'d <Self as ParsableObject<'a>>::ParserData) -> pkt::Parser<'d>;
}

pub trait ParsableObject<'a>: Sized + 'a {
    type ParserData: 'a;

    fn parse<'d>(
        data: &'d Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError>;
}

impl<'a> ParsableObject<'a> for bool {
    type ParserData = ();

    fn parse(
        _data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        match reader.read_unquoted_str() {
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(o) => Err(ArgumentNodeParseError::User(Text::translate(
                "parsing.bool.invalid",
                vec![o.into()],
            ))),
            None => Err(ArgumentNodeParseError::User(Text::translate(
                "parsing.bool.expected",
                vec![],
            ))),
        }
    }
}

impl<'a> ArgumentNode<'a> for bool {
    fn brigadier_parser<'d>(_data: &'d Self::ParserData) -> pkt::Parser<'d> {
        pkt::Parser::Bool
    }
}

macro_rules! num_argument_node {
    ($($ty: ty, $parser_i: ident, $float: expr,)*) => {
        $(
            impl<'a> ParsableObject<'a> for $ty {
                type ParserData = (Option<Self>, Option<Self>);

                fn parse(data: &Self::ParserData, reader: &mut StrReader<'a>) -> Result<Self, ArgumentNodeParseError> {

                    const EXPECTED: &'static str = {
                        if $float {
                            "parsing.float.expected"
                        } else {
                            "parsing.integer.expected"
                        }
                    };

                    const LOW: &'static str = {
                        if $float {
                            "argument.float.low"
                        } else {
                            "argument.integer.low"
                        }
                    };

                    const BIG: &'static str = {
                        if $float {
                            "argument.float.big"
                        } else {
                            "argument.integer.big"
                        }
                    };

                    match reader.read_num::<$float>() {
                        Some("") | None => Err(ArgumentNodeParseError::User(Text::translate(EXPECTED, vec![]))),
                        Some(str) => {
                            let num: Self = str.parse().map_err(|_| Text::translate(EXPECTED, vec![]))?;
                            if let Some(min) = data.0 {
                                if min > num {
                                    Err(Text::translate(LOW, vec![format!("{min}").into(), str.into()]))?;
                                }
                            }
                            if let Some(max) = data.0 {
                                if max < num {
                                    Err(Text::translate(BIG, vec![format!("{max}").into(), str.into()]))?;
                                }
                            }
                            Ok(num)
                        },
                    }
                }
            }

            impl<'a> ArgumentNode<'a> for $ty {
                fn brigadier_parser<'d>(data: &'d Self::ParserData) -> pkt::Parser<'d> {
                    pkt::Parser::$parser_i { min: data.0, max: data.0 }
                }
            }
        )*
    }
}

num_argument_node!(i32, Integer, false, i64, Long, false, f32, Float, true, f64, Double, true,);

pub use pkt::StringArg;

impl<'a> ParsableObject<'a> for Cow<'a, str> {
    type ParserData = StringArg;

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        match data {
            StringArg::SingleWord => Ok(reader
                .read_unquoted_str()
                .map(|str| Cow::Borrowed(str))
                .unwrap_or(Cow::Borrowed(&""))),
            StringArg::QuotablePhrase => {
                let ch = reader.peek_char();
                match ch {
                    Some('"') | Some('\'') => {
                        let _ = reader.next_char();
                        reader
                            .read_escaped_quoted_str()
                            .map(|str| Cow::Owned(str))
                            .ok_or_else(|| {
                                Text::translate("parsing.quote.expected.end", vec![]).into()
                            })
                    }
                    _ => Ok(reader
                        .read_unquoted_str()
                        .map(|str| Cow::Borrowed(str))
                        .unwrap_or(Cow::Borrowed(&""))),
                }
            }
            StringArg::GreedyPhrase => {
                let remaining = reader.remaining_str();
                reader.cursor_to_end();
                Ok(Cow::Borrowed(remaining))
            }
        }
    }
}

impl<'a> ArgumentNode<'a> for Cow<'a, str> {
    fn brigadier_parser<'d>(data: &'d Self::ParserData) -> pkt::Parser<'d> {
        pkt::Parser::String(*data)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Angle(pub WorldCoordinate<f32>);

impl<'a> ParsableObject<'a> for Angle {
    type ParserData = ();

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        WorldCoordinate::parse(&(Some(-180.0), Some(180.0)), reader).map(|v| Self(v))
    }
}

impl<'a> ArgumentNode<'a> for Angle {
    fn brigadier_parser<'d>(data: &'d Self::ParserData) -> pkt::Parser<'d> {
        pkt::Parser::Angle
    }
}

enum VecCArgument<const SIZE: usize, T> {
    World([WorldCoordinate<T>; SIZE]),
    Local([T; SIZE]),
}

impl<'a, const SIZE: usize, T: ParsableObject<'a>> ParsableObject<'a> for VecCArgument<SIZE, T> {
    type ParserData = T::ParserData;

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        let next_char = reader.peek_char();

        Ok(if next_char == Some('^') {
            let mut coordinates: MaybeUninit<[T; SIZE]> = MaybeUninit::uninit();
            reader.next_char();
            unsafe {
                coordinates.assume_init_mut()[0] = T::parse(data, reader)?;
            }
            for i in 1..SIZE {
                reader
                    .skip_only('^')
                    .ok_or_else(|| Text::translate(ARGUMENT_POS_MIXED, vec![]))?;
                unsafe { coordinates.assume_init_mut()[i] = T::parse(data, reader)? };
            }
            let coordinates = unsafe { coordinates.assume_init() };
            Self::Local(coordinates)
        } else {
            let mut coordinates: MaybeUninit<[WorldCoordinate<T>; SIZE]> = MaybeUninit::uninit();
            for i in 0..SIZE {
                unsafe {
                    coordinates.assume_init_mut()[i] = WorldCoordinate::parse(data, reader)?;
                }
            }
            let coordinates = unsafe { coordinates.assume_init() };
            Self::World(coordinates)
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Vec3Argument<T> {
    World {
        x: WorldCoordinate<T>,
        y: WorldCoordinate<T>,
        z: WorldCoordinate<T>,
    },
    Local {
        /// +x: Right
        /// -x: Left
        x: T,
        /// +y: Up
        /// -y: Down
        y: T,
        /// +z: Forward
        /// -z: Backwards
        z: T,
    },
}

impl<'a, T: ParsableObject<'a> + Copy> ParsableObject<'a> for Vec3Argument<T> {
    type ParserData = T::ParserData;

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        VecCArgument::<3, _>::parse(data, reader).map(|vec| match vec {
            VecCArgument::Local(local) => Self::Local {
                x: local[0],
                y: local[1],
                z: local[2],
            },
            VecCArgument::World(world) => Self::World {
                x: world[0],
                y: world[1],
                z: world[2],
            },
        })
    }
}

impl<'a, T: ParsableObject<'a> + Copy> ArgumentNode<'a> for Vec3Argument<T> {
    fn brigadier_parser<'d>(data: &'d <Self as ParsableObject<'a>>::ParserData) -> pkt::Parser<'d> {
        pkt::Parser::Vec3
    }
}

pub enum Vec2Argument<T> {
    World {
        x: WorldCoordinate<T>,
        y: WorldCoordinate<T>,
    },
    Local {
        x: T,
        y: T,
    },
}

impl<'a, T: ParsableObject<'a> + Copy> ParsableObject<'a> for Vec2Argument<T> {
    type ParserData = T::ParserData;

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        VecCArgument::<2, _>::parse(data, reader).map(|vec| match vec {
            VecCArgument::Local(local) => Self::Local {
                x: local[0],
                y: local[1],
            },
            VecCArgument::World(world) => Self::World {
                x: world[0],
                y: world[1],
            },
        })
    }
}

impl<'a, T: ParsableObject<'a> + Copy> ArgumentNode<'a> for Vec2Argument<T> {
    fn brigadier_parser<'d>(data: &'d <Self as ParsableObject<'a>>::ParserData) -> pkt::Parser<'d> {
        pkt::Parser::Vec2
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WorldCoordinate<T> {
    Relative(T),
    Absolute(T),
}

impl<'a, T: ParsableObject<'a>> ParsableObject<'a> for WorldCoordinate<T> {
    type ParserData = T::ParserData;

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        let ch = reader.peek_char();
        if ch == Some('^') {
            Err(Text::translate(ARGUMENT_POS_MIXED, vec![]))?;
        }
        let relative = if ch == Some('~') {
            reader.next_char();
            true
        } else {
            false
        };
        let num = T::parse(data, reader)?;
        Ok(if relative {
            Self::Relative(num)
        } else {
            Self::Absolute(num)
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockPosArgument(Vec3Argument<i32>);

impl<'a> ParsableObject<'a> for BlockPosArgument {
    type ParserData = ();

    fn parse(
        data: &Self::ParserData,
        reader: &mut StrReader<'a>,
    ) -> Result<Self, ArgumentNodeParseError> {
        Vec3Argument::parse(&(None, None), reader).map(|v| Self(v))
    }
}

impl<'a> ArgumentNode<'a> for BlockPosArgument {
    fn brigadier_parser<'d>(data: &'d <Self as ParsableObject<'_>>::ParserData) -> pkt::Parser<'d> {
        pkt::Parser::BlockPos
    }
}
