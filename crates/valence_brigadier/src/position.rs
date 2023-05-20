use std::mem::MaybeUninit;
use std::ops::Add;

use glam::{DVec3, IVec3, Vec3};
use valence_core::packet::s2c::play::command_tree::Parser;
use valence_core::translation_key::COMMAND_EXPECTED_SEPARATOR;
use valence_entity::Look;

use crate::parser::{BrigadierArgument, ErrorMessage, Parsable, ParsingError};
use crate::reader::StrReader;

pub const POS_MIXED: &str = "argument.pos.mixed";
pub const ANGLE_INVALID: &str = "argument.angle.invalid";

pub const ANGLE_BOUNDS: (f32, f32) = (-180.0, 180.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angle(pub WorldCoordinate<f32>);

impl<'a> Parsable<'a> for Angle {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let relative = reader.skip_only('~').is_some();
        let angle = f32::parse(&(f32::MIN, f32::MAX), reader)?;

        if (-180.0..=180.0).contains(&angle) {
            Err(ANGLE_INVALID.empty())?;
        }

        Ok(Self(if relative {
            WorldCoordinate::Relative(angle)
        } else {
            WorldCoordinate::Absolute(angle)
        }))
    }
}

impl<'a> BrigadierArgument<'a> for Angle {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Angle
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldCoordinate<T> {
    Relative(T),
    Absolute(T),
}

impl<T: Add<T, Output = T> + Clone> WorldCoordinate<T> {
    /// Applies given coordinate value. Usually an executor's coordinate
    pub fn apply(&self, value: T) -> T {
        match self {
            Self::Absolute(v) => v.clone(),
            Self::Relative(v) => v.clone() + value,
        }
    }
}

impl<'a, T: Parsable<'a>> Parsable<'a> for WorldCoordinate<T> {
    type Data = T::Data;

    fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let relative = match reader.peek_char() {
            Some('~') => {
                let _ = reader.next_char();
                true
            }
            Some('^') => Err(POS_MIXED.empty())?,
            _ => false,
        };
        let value = T::parse(data, reader)?;
        Ok(if relative {
            Self::Relative(value)
        } else {
            Self::Absolute(value)
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Vec3Argument<T> {
    Local([WorldCoordinate<T>; 3]),
    Relative([T; 3]),
}

macro_rules! vec3_impl {
    ($($ty: ty, $vec_ty: ty,)*) => {
        $(impl Vec3Argument<$ty> {
            pub fn apply(&self, pos: DVec3, look: Look) -> $vec_ty {
                match self {
                    Self::Local(local) => <$vec_ty>::new(
                        local[0].apply(pos[0] as $ty),
                        local[1].apply(pos[1] as $ty),
                        local[2].apply(pos[2] as $ty),
                    ),
                    Self::Relative(relative) => {
                        let relative: $vec_ty = (*relative).into();
                        let look_vec = look.vec();
                        <$vec_ty>::new(look_vec.x as _, look_vec.y as _, look_vec.z as _)
                            * relative
                            + <$vec_ty>::new(pos.x as _, pos.y as _, pos.z as _)
                    }
                }
            }
        })*
    }
}

vec3_impl!(f64, DVec3, f32, Vec3, i32, IVec3,);

impl<'a, T: Parsable<'a>> Parsable<'a> for Vec3Argument<T> {
    type Data = [T::Data; 3];

    #[allow(invalid_value)]
    #[allow(clippy::uninit_assumed_init)]
    fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        if reader.skip_only('^').is_some() {
            // SAFETY.
            // We only write into the array, we are not reading it.
            let mut result: [T; 3] = unsafe { MaybeUninit::uninit().assume_init() };
            result[0] = T::parse(&data[0], reader)?;
            for i in 1..3 {
                if reader.skip_only(' ').is_none() {
                    Err(COMMAND_EXPECTED_SEPARATOR.empty())?;
                }
                if reader.skip_only('^').is_none() {
                    Err(POS_MIXED.empty())?;
                }
                result[i] = T::parse(&data[i], reader)?;
            }
            Ok(Self::Relative(result))
        } else {
            // SAFETY.
            // We only write into the array, we are not reading it.
            let mut result: [WorldCoordinate<T>; 3] =
                unsafe { MaybeUninit::uninit().assume_init() };
            result[0] = WorldCoordinate::parse(&data[0], reader)?;
            for i in 1..3 {
                if reader.skip_only(' ').is_none() {
                    Err(COMMAND_EXPECTED_SEPARATOR.empty())?;
                }
                result[i] = WorldCoordinate::parse(&data[i], reader)?;
            }
            Ok(Self::Local(result))
        }
    }
}

impl<'a> BrigadierArgument<'a> for Vec3Argument<i32> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::BlockPos
    }
}

impl<'a> BrigadierArgument<'a> for Vec3Argument<f32> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Vec3
    }
}

impl<'a> BrigadierArgument<'a> for Vec3Argument<f64> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Vec3
    }
}

pub const BLOCK_POS_DATA: [(i32, i32); 3] = [
    (-30_000_000, 30_000_000),
    (-64, 320),
    (-30_000_000, 30_000_000),
];

pub struct BlockPosArgument(pub Vec3Argument<i32>);

impl<'a> Parsable<'a> for BlockPosArgument {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        Ok(Self(Vec3Argument::parse(&BLOCK_POS_DATA, reader)?))
    }
}

impl<'a> BrigadierArgument<'a> for BlockPosArgument {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::BlockPos
    }
}

pub struct Vec2Argument<T>(pub [WorldCoordinate<T>; 2]);

impl<'a, T: Parsable<'a>> Parsable<'a> for Vec2Argument<T> {
    type Data = [T::Data; 2];

    #[allow(invalid_value)]
    #[allow(clippy::uninit_assumed_init)]
    fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        // SAFETY.
        // We only write into the array, we are not reading it.
        let mut result: [WorldCoordinate<T>; 2] = unsafe { MaybeUninit::uninit().assume_init() };

        result[0] = WorldCoordinate::parse(&data[0], reader)?;
        if reader.skip_only(' ').is_none() {
            Err(COMMAND_EXPECTED_SEPARATOR.empty())?;
        }
        result[1] = WorldCoordinate::parse(&data[1], reader)?;

        Ok(Self(result))
    }
}

impl<'a> BrigadierArgument<'a> for Vec2Argument<i32> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::ColumnPos
    }
}

impl<'a> BrigadierArgument<'a> for Vec2Argument<f32> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Vec2
    }
}

impl<'a> BrigadierArgument<'a> for Vec2Argument<f64> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Vec2
    }
}
