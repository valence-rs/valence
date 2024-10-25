use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct CustomReportDetailsS2c<'a> {
    pub details: Vec<CustomReportDetail<'a>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CustomReportDetail<'a> {
    pub title: Bounded<&'a str, 128>,
    pub description: Bounded<&'a str, 4096>,
}
