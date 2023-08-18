use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}
