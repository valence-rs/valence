use std::io::Write;

use anyhow::Context;
use vek::Vec3;

use crate::block::BlockState;
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
        r: f32,
        g: f32,
        b: f32,
        scale: f32,
    },
    DustColorTransition {
        from_r: f32,
        from_g: f32,
        from_b: f32,
        scale: f32,
        to_r: f32,
        to_g: f32,
        to_b: f32,
    },
    Effect,
    ElderGuardian,
    EnchantedHit,
    Enchant,
    EndRod,
    EntityEffect,
    ExplosionEmitter,
    Explosion,
    FallingDust(BlockState),
    Firework,
    Fishing,
    Flame,
    SoulFireFlame,
    Soul,
    Flash,
    HappyVillager,
    Composter,
    Heart,
    InstantEffect,
    Item(u32), // TODO: field is 'Slot': 'The item that will be used.'
    Vibration {
        position_source_type: String,
        block_pos: Vec3<f32>,
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
            ParticleType::FallingDust(_) => 24,
            ParticleType::Firework => 25,
            ParticleType::Fishing => 26,
            ParticleType::Flame => 27,
            ParticleType::SoulFireFlame => 28,
            ParticleType::Soul => 29,
            ParticleType::Flash => 30,
            ParticleType::HappyVillager => 31,
            ParticleType::Composter => 32,
            ParticleType::Heart => 33,
            ParticleType::InstantEffect => 34,
            ParticleType::Item(_) => 35,
            ParticleType::Vibration { .. } => 36,
            ParticleType::ItemSlime => 37,
            ParticleType::ItemSnowball => 38,
            ParticleType::LargeSmoke => 39,
            ParticleType::Lava => 40,
            ParticleType::Mycelium => 41,
            ParticleType::Note => 42,
            ParticleType::Poof => 43,
            ParticleType::Portal => 44,
            ParticleType::Rain => 45,
            ParticleType::Smoke => 46,
            ParticleType::Sneeze => 47,
            ParticleType::Spit => 48,
            ParticleType::SquidInk => 49,
            ParticleType::SweepAttack => 50,
            ParticleType::TotemOfUndying => 51,
            ParticleType::Underwater => 52,
            ParticleType::Splash => 53,
            ParticleType::Witch => 54,
            ParticleType::BubblePop => 55,
            ParticleType::CurrentDown => 56,
            ParticleType::BubbleColumnUp => 57,
            ParticleType::Nautilus => 58,
            ParticleType::Dolphin => 59,
            ParticleType::CampfireCosySmoke => 60,
            ParticleType::CampfireSignalSmoke => 61,
            ParticleType::DrippingHoney => 62,
            ParticleType::FallingHoney => 63,
            ParticleType::LandingHoney => 64,
            ParticleType::FallingNectar => 65,
            ParticleType::FallingSporeBlossom => 66,
            ParticleType::Ash => 67,
            ParticleType::CrimsonSpore => 68,
            ParticleType::WarpedSpore => 69,
            ParticleType::SporeBlossomAir => 70,
            ParticleType::DrippingObsidianTear => 71,
            ParticleType::FallingObsidianTear => 72,
            ParticleType::LandingObsidianTear => 73,
            ParticleType::ReversePortal => 74,
            ParticleType::WhiteAsh => 75,
            ParticleType::SmallFlame => 76,
            ParticleType::Snowflake => 77,
            ParticleType::DrippingDripstoneLava => 78,
            ParticleType::FallingDripstoneLava => 79,
            ParticleType::DrippingDripstoneWater => 80,
            ParticleType::FallingDripstoneWater => 81,
            ParticleType::GlowSquidInk => 82,
            ParticleType::Glow => 83,
            ParticleType::WaxOn => 84,
            ParticleType::WaxOff => 85,
            ParticleType::ElectricSpark => 86,
            ParticleType::Scrape => 87,
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
            stringify!(particawdle_type),
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
            ParticleType::Dust { r, g, b, scale } => {
                Encode::encode(r, _w).context(concat!(
                    "failed to write field `",
                    stringify!(r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(g, _w).context(concat!(
                    "failed to write field `",
                    stringify!(g),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(b, _w).context(concat!(
                    "failed to write field `",
                    stringify!(b),
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
                from_r,
                from_g,
                from_b,
                scale,
                to_r,
                to_g,
                to_b,
            } => {
                Encode::encode(from_r, _w).context(concat!(
                    "failed to write field `",
                    stringify!(from_r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(from_g, _w).context(concat!(
                    "failed to write field `",
                    stringify!(from_g),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(from_b, _w).context(concat!(
                    "failed to write field `",
                    stringify!(from_b),
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
                Encode::encode(to_r, _w).context(concat!(
                    "failed to write field `",
                    stringify!(to_r),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(to_g, _w).context(concat!(
                    "failed to write field `",
                    stringify!(to_g),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(to_b, _w).context(concat!(
                    "failed to write field `",
                    stringify!(to_b),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::FallingDust(block_state) => {
                Encode::encode(block_state, _w).context(concat!(
                    "failed to write field `",
                    stringify!(block_state),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
            }
            ParticleType::Item(_) => todo!("Item particle not yet implemented"),
            ParticleType::Vibration {
                position_source_type,
                block_pos,
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                Encode::encode(position_source_type, _w).context(concat!(
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
                Encode::encode(entity_id, _w).context(concat!(
                    "failed to write field `",
                    stringify!(entity_id),
                    "` from struct `",
                    stringify!(Particle),
                    "`"
                ))?;
                Encode::encode(entity_eye_height, _w).context(concat!(
                    "failed to write field `",
                    stringify!(entity_eye_height),
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
            ParticleType::Dust { r, g, b, scale } => {
                r.encoded_len() + g.encoded_len() + b.encoded_len() + scale.encoded_len()
            }
            ParticleType::DustColorTransition {
                from_r,
                from_g,
                from_b,
                scale,
                to_r,
                to_g,
                to_b,
            } => {
                from_r.encoded_len()
                    + from_g.encoded_len()
                    + from_b.encoded_len()
                    + scale.encoded_len()
                    + to_r.encoded_len()
                    + to_g.encoded_len()
                    + to_b.encoded_len()
            }
            ParticleType::FallingDust(block_state) => block_state.encoded_len(),
            ParticleType::Item(_) => todo!("Item particle not yet implemented"),
            ParticleType::Vibration {
                position_source_type,
                block_pos,
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                position_source_type.encoded_len()
                    + block_pos.encoded_len()
                    + entity_id.encoded_len()
                    + entity_eye_height.encoded_len()
                    + ticks.encoded_len()
            }
            _ => 0,
        };
        id_len + data_len
    }
}
