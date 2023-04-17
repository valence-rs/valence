use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct ScoreboardObjectiveUpdateS2c<'a> {
    pub objective_name: &'a str,
    pub mode: Mode,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Mode {
    Create {
        objective_display_name: Text,
        render_type: RenderType,
    },
    Remove,
    Update {
        objective_display_name: Text,
        render_type: RenderType,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum RenderType {
    Integer,
    Hearts,
}
