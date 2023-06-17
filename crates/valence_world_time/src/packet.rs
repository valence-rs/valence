use valence_core::protocol::{packet_id, Decode, Encode, Packet};


#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_TIME_UPDATE_S2C)]
pub struct WorldTimeUpdateS2c {
    /// The age of the world in 1/20ths of a second.
    pub world_age: i64,
    /// The current time of day in 1/20ths of a second.
    /// The value should be in the range \[0, 24000].
    /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
    pub time_of_day: i64,
}
