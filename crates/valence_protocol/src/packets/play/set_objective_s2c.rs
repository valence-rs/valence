use std::borrow::Cow;

use bevy_ecs::prelude::*;
use valence_nbt::Compound;
use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetObjectiveS2c<'a> {
    pub objective_name: &'a str,
    pub mode: ObjectiveMode<'a>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum ObjectiveMode<'a> {
    Create {
        objective_display_name: Cow<'a, Text>,
        render_type: ObjectiveRenderType,
        number_format: Option<NumberFormat<'a>>,
    },
    Remove,
    Update {
        objective_display_name: Cow<'a, Text>,
        render_type: ObjectiveRenderType,
        number_format: Option<NumberFormat<'a>>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Component, Default)]
pub enum ObjectiveRenderType {
    /// Display the value as a number.
    #[default]
    Integer,
    /// Display the value as hearts.
    Hearts,
}
#[derive(Clone, PartialEq, Debug, Encode, Decode, Component)]
pub enum NumberFormat<'a> {
    Blank,
    Styled { styling: Compound },
    Fixed { content: Cow<'a, Text> },
}
