use super::*;

// #[derive(Clone, Debug, Encode, Decode, Packet)]
// #[packet(id = packet_id::BOSS_BAR_S2C)]
// pub struct BossBarS2c<'a> {
//     pub id: Uuid,
//     pub action: BossBarAction<'a>,
// }

// #[derive(Clone, PartialEq, Debug, Encode, Decode)]
// pub enum BossBarAction<'a> {
//     Add {
//         title: Cow<'a, Text>,
//         health: f32,
//         color: BossBarColor,
//         division: BossBarDivision,
//         flags: BossBarFlags,
//     },
//     Remove,
//     UpdateHealth(f32),
//     UpdateTitle(Cow<'a, Text>),
//     UpdateStyle(BossBarColor, BossBarDivision),
//     UpdateFlags(BossBarFlags),
// }
