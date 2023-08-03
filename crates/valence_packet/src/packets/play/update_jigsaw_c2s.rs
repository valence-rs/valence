use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_JIGSAW_C2S)]
pub struct UpdateJigsawC2s<'a> {
    pub position: BlockPos,
    pub name: Ident<Cow<'a, str>>,
    pub target: Ident<Cow<'a, str>>,
    pub pool: Ident<Cow<'a, str>>,
    pub final_state: &'a str,
    pub joint_type: &'a str,
}
