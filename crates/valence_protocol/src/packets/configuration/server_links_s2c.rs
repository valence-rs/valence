use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct ServerLinksS2c<'a> {
    pub links: Vec<ServerLink<'a>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ServerLink<'a> {
    pub label: ServerLinkEnum,
    pub url: Cow<'a, str>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum ServerLinkEnum {
    BuiltIn(BuiltInLinkType),
    CustomText(Text),
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum BuiltInLinkType {
    BugReport,
    CommunityGuidelines,
    Support,
    Status,
    Feedback,
    Community,
    Website,
    Forums,
    News,
    Announcements,
}
