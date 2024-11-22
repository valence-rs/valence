use bitfield_struct::bitfield;

use crate::packets::configuration::client_information_c2s::ParticleMode;
use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ClientInformationC2s<'a> {
    pub locale: &'a str,
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub main_arm: MainArm,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
    pub particle_mode: ParticleMode,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct DisplayedSkinParts {
    pub cape: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants_leg: bool,
    pub right_pants_leg: bool,
    pub hat: bool,
    _pad: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    #[default]
    Hidden,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Encode, Decode)]
pub enum MainArm {
    Left,
    #[default]
    Right,
}
