use std::io::Write;

use anyhow::Context;
use vek::Vec3;

use crate::block::{BlockPos, BlockState};
use crate::protocol::{Decode, Encode, VarInt};

#[derive(Clone, Debug)]
pub struct Particle {
    pub particle_type: ParticleType,
    pub long_distance: bool,
    pub position: Vec3<f64>,
    pub offset: Vec3<f32>,
    pub max_speed: f32,
    pub particle_count: u32,
}

#[derive(Clone, Debug)]
pub enum ParticleType {
    AmbientEntityEffect,
    AngryVillager,
    Block(BlockState),
    BlockMarker(BlockState),
    Bubble,
    Cloud,
    Crit,
    DamageIndicator,
    DragonBreath,
    DrippingLava,
    FallingLava,
    LandingLava,
    DrippingWater,
    FallingWater,
    Dust {
        rgb: Vec3<f32>,
        scale: f32,
    },
    DustColorTransition {
        from_rgb: Vec3<f32>,
        scale: f32,
        to_rgb: Vec3<f32>,
    },
    Effect,
    ElderGuardian,
    EnchantedHit,
    Enchant,
    EndRod,
    EntityEffect,
    ExplosionEmitter,
    Explosion,
    SonicBoom,
    FallingDust(BlockState),
    Firework,
    Fishing,
    Flame,
    SculkSoul,
    SculkCharge {
        roll: f32,
    },
    SculkChargePop,
    SoulFireFlame,
    Soul,
    Flash,
    HappyVillager,
    Composter,
    Heart,
    InstantEffect,
    Item(u32), // TODO: field is 'Slot': 'The item that will be used.'
    VibrationBlock { // The 'Block' variant of the 'Vibration' particle
        block_pos: BlockPos,
        ticks: VarInt
    },
    VibrationEntity { // The 'Entity' variant of the 'Vibration' particle
        entity_id: VarInt,
        entity_eye_height: f32,
        ticks: VarInt,
    },
    ItemSlime,
    ItemSnowball,
    LargeSmoke,
    Lava,
    Mycelium,
    Note,
    Poof,
    Portal,
    Rain,
    Smoke,
    Sneeze,
    Spit,
    SquidInk,
    SweepAttack,
    TotemOfUndying,
    Underwater,
    Splash,
    Witch,
    BubblePop,
    CurrentDown,
    BubbleColumnUp,
    Nautilus,
    Dolphin,
    CampfireCosySmoke,
    CampfireSignalSmoke,
    DrippingHoney,
    FallingHoney,
    LandingHoney,
    FallingNectar,
    FallingSporeBlossom,
    Ash,
    CrimsonSpore,
    WarpedSpore,
    SporeBlossomAir,
    DrippingObsidianTear,
    FallingObsidianTear,
    LandingObsidianTear,
    ReversePortal,
    WhiteAsh,
    SmallFlame,
    Snowflake,
    DrippingDripstoneLava,
    FallingDripstoneLava,
    DrippingDripstoneWater,
    FallingDripstoneWater,
    GlowSquidInk,
    Glow,
    WaxOn,
    WaxOff,
    ElectricSpark,
    Scrape,
}

impl ParticleType {
    fn id(&self) -> i32 {
        match self {
            ParticleType::AmbientEntityEffect => 0,
            ParticleType::AngryVillager => 1,
            ParticleType::Block(_) => 2,
            ParticleType::BlockMarker(_) => 3,
            ParticleType::Bubble => 4,
            ParticleType::Cloud => 5,
            ParticleType::Crit => 6,
            ParticleType::DamageIndicator => 7,
            ParticleType::DragonBreath => 8,
            ParticleType::DrippingLava => 9,
            ParticleType::FallingLava => 10,
            ParticleType::LandingLava => 11,
            ParticleType::DrippingWater => 12,
            ParticleType::FallingWater => 13,
            ParticleType::Dust { .. } => 14,
            ParticleType::DustColorTransition { .. } => 15,
            ParticleType::Effect => 16,
            ParticleType::ElderGuardian => 17,
            ParticleType::EnchantedHit => 18,
            ParticleType::Enchant => 19,
            ParticleType::EndRod => 20,
            ParticleType::EntityEffect => 21,
            ParticleType::ExplosionEmitter => 22,
            ParticleType::Explosion => 23,
            ParticleType::SonicBoom => 24,
            ParticleType::FallingDust(_) => 25,
            ParticleType::Firework => 26,
            ParticleType::Fishing => 27,
            ParticleType::Flame => 28,
            ParticleType::SculkSoul => 29,
            ParticleType::SculkCharge{ .. } => 30,
            ParticleType::SculkChargePop => 31,
            ParticleType::SoulFireFlame => 32,
            ParticleType::Soul => 33,
            ParticleType::Flash => 34,
            ParticleType::HappyVillager => 35,
            ParticleType::Composter => 36,
            ParticleType::Heart => 37,
            ParticleType::InstantEffect => 38,
            ParticleType::Item(_) => 39,
            ParticleType::VibrationBlock { .. } => 40,
            ParticleType::VibrationEntity { .. } => 40,
            ParticleType::ItemSlime => 41,
            ParticleType::ItemSnowball => 42,
            ParticleType::LargeSmoke => 43,
            ParticleType::Lava => 44,
            ParticleType::Mycelium => 45,
            ParticleType::Note => 46,
            ParticleType::Poof => 47,
            ParticleType::Portal => 48,
            ParticleType::Rain => 49,
            ParticleType::Smoke => 50,
            ParticleType::Sneeze => 51,
            ParticleType::Spit => 52,
            ParticleType::SquidInk => 53,
            ParticleType::SweepAttack => 54,
            ParticleType::TotemOfUndying => 55,
            ParticleType::Underwater => 56,
            ParticleType::Splash => 57,
            ParticleType::Witch => 58,
            ParticleType::BubblePop => 59,
            ParticleType::CurrentDown => 60,
            ParticleType::BubbleColumnUp => 61,
            ParticleType::Nautilus => 62,
            ParticleType::Dolphin => 63,
            ParticleType::CampfireCosySmoke => 64,
            ParticleType::CampfireSignalSmoke => 65,
            ParticleType::DrippingHoney => 66,
            ParticleType::FallingHoney => 67,
            ParticleType::LandingHoney => 68,
            ParticleType::FallingNectar => 69,
            ParticleType::FallingSporeBlossom => 70,
            ParticleType::Ash => 71,
            ParticleType::CrimsonSpore => 72,
            ParticleType::WarpedSpore => 73,
            ParticleType::SporeBlossomAir => 74,
            ParticleType::DrippingObsidianTear => 75,
            ParticleType::FallingObsidianTear => 76,
            ParticleType::LandingObsidianTear => 77,
            ParticleType::ReversePortal => 78,
            ParticleType::WhiteAsh => 79,
            ParticleType::SmallFlame => 80,
            ParticleType::Snowflake => 81,
            ParticleType::DrippingDripstoneLava => 82,
            ParticleType::FallingDripstoneLava => 83,
            ParticleType::DrippingDripstoneWater => 84,
            ParticleType::FallingDripstoneWater => 85,
            ParticleType::GlowSquidInk => 86,
            ParticleType::Glow => 87,
            ParticleType::WaxOn => 88,
            ParticleType::WaxOff => 89,
            ParticleType::ElectricSpark => 90,
            ParticleType::Scrape => 91,
        }
    }
}

impl Encode for Particle {
    fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
        let particle_id: VarInt = VarInt(self.particle_type.id());
        Encode::encode(&particle_id, _w).context(concat!(
            "failed to write field `",
            stringify!(particle_id),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Encode::encode(&self.long_distance, _w).context(concat!(
            "failed to write field `",
            stringify!(long_distance),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Encode::encode(&self.position, _w).context(concat!(
            "failed to write field `",
            stringify!(position),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Encode::encode(&self.offset, _w).context(concat!(
            "failed to write field `",
            stringify!(offset),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Encode::encode(&self.max_speed, _w).context(concat!(
            "failed to write field `",
            stringify!(max_speed),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Encode::encode(&self.particle_count, _w).context(concat!(
            "failed to write field `",
            stringify!(particle_count),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Encode::encode(&self.particle_type, _w).context(concat!(
            "failed to write field `",
            stringify!(particle_type),
            "` from struct `",
            stringify!(Particle),
            "`"
        ))?;
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.particle_type.encoded_len()
            + self.long_distance.encoded_len()
            + self.position.encoded_len()
            + self.offset.encoded_len()
            + self.max_speed.encoded_len()
            + self.particle_count.encoded_len()
    }
}

impl Decode for Particle {
    fn decode(_r: &mut &[u8]) -> anyhow::Result<Self> {
        todo!("Is this even necessary?");
        // let particle_id: VarInt = Decode::decode(_r).context(concat!("failed
        // to read field `", stringify!(particle_id), "` from struct `",
        // stringify!(Particle), "`"))?;
    }
}

impl Encode for ParticleType {
    fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
        match self {
            ParticleType::Block(block_state) => {
                Encode::encode(block_state, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_state),
                    "` from struct `",
                    stringify!(ParticleType),
                    "`"
                ))?;
            }
            ParticleType::BlockMarker(block_state) => {
                Encode::encode(block_state, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_state),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::Dust { rgb, scale } => {
                Encode::encode(rgb, _w).context(concat!(
                    "failed to write field `",
                    stringify!(r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(scale, _w).context(concat!(
                    "failed to write field `",
                    stringify!(scale),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::DustColorTransition {
                from_rgb,
                scale,
                to_rgb,
            } => {
                Encode::encode(from_rgb, _w).context(concat!(
                    "failed to write field `",
                    stringify!(from_r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(scale, _w).context(concat!(
                    "failed to write field `",
                    stringify!(scale),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(to_rgb, _w).context(concat!(
                    "failed to write field `",
                    stringify!(to_r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::FallingDust(block_state) => {
                Encode::encode(block_state, _w).context(concat!(
                    "failed to write field `",
                    stringify!(to_r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::SculkCharge {
                roll,
            } => {
                Encode::encode(roll, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_pos),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::Item(_) => todo!("Item particle not yet implemented"),
            ParticleType::VibrationBlock {
                block_pos,
                ticks,
            } => {
                Encode::encode(&"block".to_string(), _w).context(concat!(
                    "failed to write field `",
                    stringify!(position_source_type),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(block_pos, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_pos),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(ticks, _w).context(concat!(
                    "failed to write field `",
                    stringify!(ticks),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                Encode::encode(&"entity".to_string(), _w).context(concat!(
                    "failed to write field `",
                    stringify!(position_source_type),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(entity_id, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_pos),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(entity_eye_height, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_pos),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(ticks, _w).context(concat!(
                    "failed to write field `",
                    stringify!(ticks),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            _ => {}
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let id_len = VarInt(self.id()).encoded_len();
        let data_len = match self {
            ParticleType::Block(block_state) => block_state.encoded_len(),
            ParticleType::BlockMarker(block_state) => block_state.encoded_len(),
            ParticleType::Dust { .. } => 4 * 4,
            ParticleType::DustColorTransition { .. } => 7 * 4,
            ParticleType::FallingDust(block_state) => block_state.encoded_len(),
            ParticleType::SculkCharge { .. } => 4,
            ParticleType::Item(_) => todo!("Item particle not yet implemented"),
            ParticleType::VibrationBlock {
                block_pos,
                ticks,
            } => {
                &"block".to_string().encoded_len()
                    + block_pos.encoded_len()
                    + ticks.encoded_len()
            }
            ParticleType::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                &"entity".to_string().encoded_len()
                    + entity_id.encoded_len()
                    + entity_eye_height.encoded_len()
                    + ticks.encoded_len()
            }
            _ => 0,
        };
        id_len + data_len
    }
}
