use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;

use crate::block::BlockState;
use crate::block_pos::BlockPos;
use crate::item::ItemStack;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug)]
pub struct ParticleS2c<'a> {
    pub particle: Cow<'a, Particle>,
    pub long_distance: bool,
    pub position: [f64; 3],
    pub offset: [f32; 3],
    pub max_speed: f32,
    pub count: i32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Particle {
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
        rgb: [f32; 3],
        scale: f32,
    },
    DustColorTransition {
        from_rgb: [f32; 3],
        scale: f32,
        to_rgb: [f32; 3],
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
    DrippingCherryLeaves,
    FallingCherryLeaves,
    LandingCherryLeaves,
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
    Item(Option<ItemStack>),
    /// The 'Block' variant of the 'Vibration' particle
    VibrationBlock {
        block_pos: BlockPos,
        ticks: i32,
    },
    /// The 'Entity' variant of the 'Vibration' particle
    VibrationEntity {
        entity_id: i32,
        entity_eye_height: f32,
        ticks: i32,
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
    Shriek {
        delay: i32,
    },
}

impl Particle {
    pub const fn id(&self) -> i32 {
        match self {
            Particle::AmbientEntityEffect => 0,
            Particle::AngryVillager => 1,
            Particle::Block(_) => 2,
            Particle::BlockMarker(_) => 3,
            Particle::Bubble => 4,
            Particle::Cloud => 5,
            Particle::Crit => 6,
            Particle::DamageIndicator => 7,
            Particle::DragonBreath => 8,
            Particle::DrippingLava => 9,
            Particle::FallingLava => 10,
            Particle::LandingLava => 11,
            Particle::DrippingWater => 12,
            Particle::FallingWater => 13,
            Particle::Dust { .. } => 14,
            Particle::DustColorTransition { .. } => 15,
            Particle::Effect => 16,
            Particle::ElderGuardian => 17,
            Particle::EnchantedHit => 18,
            Particle::Enchant => 19,
            Particle::EndRod => 20,
            Particle::EntityEffect => 21,
            Particle::ExplosionEmitter => 22,
            Particle::Explosion => 23,
            Particle::SonicBoom => 24,
            Particle::FallingDust(_) => 25,
            Particle::Firework => 26,
            Particle::Fishing => 27,
            Particle::Flame => 28,
            Particle::DrippingCherryLeaves => 29,
            Particle::FallingCherryLeaves => 30,
            Particle::LandingCherryLeaves => 31,
            Particle::SculkSoul => 32,
            Particle::SculkCharge { .. } => 33,
            Particle::SculkChargePop => 34,
            Particle::SoulFireFlame => 35,
            Particle::Soul => 36,
            Particle::Flash => 37,
            Particle::HappyVillager => 38,
            Particle::Composter => 39,
            Particle::Heart => 40,
            Particle::InstantEffect => 41,
            Particle::Item { .. } => 42,
            Particle::VibrationBlock { .. } => 43,
            Particle::VibrationEntity { .. } => 43,
            Particle::ItemSlime => 44,
            Particle::ItemSnowball => 45,
            Particle::LargeSmoke => 46,
            Particle::Lava => 47,
            Particle::Mycelium => 48,
            Particle::Note => 49,
            Particle::Poof => 50,
            Particle::Portal => 51,
            Particle::Rain => 52,
            Particle::Smoke => 53,
            Particle::Sneeze => 54,
            Particle::Spit => 55,
            Particle::SquidInk => 56,
            Particle::SweepAttack => 57,
            Particle::TotemOfUndying => 58,
            Particle::Underwater => 59,
            Particle::Splash => 60,
            Particle::Witch => 61,
            Particle::BubblePop => 62,
            Particle::CurrentDown => 63,
            Particle::BubbleColumnUp => 64,
            Particle::Nautilus => 65,
            Particle::Dolphin => 66,
            Particle::CampfireCosySmoke => 67,
            Particle::CampfireSignalSmoke => 68,
            Particle::DrippingHoney => 69,
            Particle::FallingHoney => 70,
            Particle::LandingHoney => 71,
            Particle::FallingNectar => 72,
            Particle::FallingSporeBlossom => 73,
            Particle::Ash => 74,
            Particle::CrimsonSpore => 75,
            Particle::WarpedSpore => 76,
            Particle::SporeBlossomAir => 77,
            Particle::DrippingObsidianTear => 78,
            Particle::FallingObsidianTear => 79,
            Particle::LandingObsidianTear => 80,
            Particle::ReversePortal => 81,
            Particle::WhiteAsh => 82,
            Particle::SmallFlame => 83,
            Particle::Snowflake => 84,
            Particle::DrippingDripstoneLava => 85,
            Particle::FallingDripstoneLava => 86,
            Particle::DrippingDripstoneWater => 87,
            Particle::FallingDripstoneWater => 88,
            Particle::GlowSquidInk => 89,
            Particle::Glow => 90,
            Particle::WaxOn => 91,
            Particle::WaxOff => 92,
            Particle::ElectricSpark => 93,
            Particle::Scrape => 94,
            Particle::Shriek { .. } => 95,
        }
    }

    /// Decodes the particle assuming the given particle ID.
    pub fn decode_with_id(particle_id: i32, r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(match particle_id {
            0 => Particle::AmbientEntityEffect,
            1 => Particle::AngryVillager,
            2 => Particle::Block(BlockState::decode(r)?),
            3 => Particle::BlockMarker(BlockState::decode(r)?),
            4 => Particle::Bubble,
            5 => Particle::Cloud,
            6 => Particle::Crit,
            7 => Particle::DamageIndicator,
            8 => Particle::DragonBreath,
            9 => Particle::DrippingLava,
            10 => Particle::FallingLava,
            11 => Particle::LandingLava,
            12 => Particle::DrippingWater,
            13 => Particle::FallingWater,
            14 => Particle::Dust {
                rgb: <[f32; 3]>::decode(r)?,
                scale: f32::decode(r)?,
            },
            15 => Particle::DustColorTransition {
                from_rgb: <[f32; 3]>::decode(r)?,
                scale: f32::decode(r)?,
                to_rgb: <[f32; 3]>::decode(r)?,
            },
            16 => Particle::Effect,
            17 => Particle::ElderGuardian,
            18 => Particle::EnchantedHit,
            19 => Particle::Enchant,
            20 => Particle::EndRod,
            21 => Particle::EntityEffect,
            22 => Particle::ExplosionEmitter,
            23 => Particle::Explosion,
            24 => Particle::SonicBoom,
            25 => Particle::FallingDust(BlockState::decode(r)?),
            26 => Particle::Firework,
            27 => Particle::Fishing,
            28 => Particle::Flame,
            29 => Particle::DrippingCherryLeaves,
            30 => Particle::FallingCherryLeaves,
            31 => Particle::LandingCherryLeaves,
            32 => Particle::SculkSoul,
            33 => Particle::SculkCharge {
                roll: f32::decode(r)?,
            },
            34 => Particle::SculkChargePop,
            35 => Particle::SoulFireFlame,
            36 => Particle::Soul,
            37 => Particle::Flash,
            38 => Particle::HappyVillager,
            39 => Particle::Composter,
            40 => Particle::Heart,
            41 => Particle::InstantEffect,
            42 => Particle::Item(Decode::decode(r)?),
            43 => match <&str>::decode(r)? {
                "block" => Particle::VibrationBlock {
                    block_pos: BlockPos::decode(r)?,
                    ticks: VarInt::decode(r)?.0,
                },
                "entity" => Particle::VibrationEntity {
                    entity_id: VarInt::decode(r)?.0,
                    entity_eye_height: f32::decode(r)?,
                    ticks: VarInt::decode(r)?.0,
                },
                invalid => bail!("invalid vibration position source of \"{invalid}\""),
            },
            44 => Particle::ItemSlime,
            45 => Particle::ItemSnowball,
            46 => Particle::LargeSmoke,
            47 => Particle::Lava,
            48 => Particle::Mycelium,
            49 => Particle::Note,
            50 => Particle::Poof,
            51 => Particle::Portal,
            52 => Particle::Rain,
            53 => Particle::Smoke,
            54 => Particle::Sneeze,
            55 => Particle::Spit,
            56 => Particle::SquidInk,
            57 => Particle::SweepAttack,
            58 => Particle::TotemOfUndying,
            59 => Particle::Underwater,
            60 => Particle::Splash,
            61 => Particle::Witch,
            62 => Particle::BubblePop,
            63 => Particle::CurrentDown,
            64 => Particle::BubbleColumnUp,
            65 => Particle::Nautilus,
            66 => Particle::Dolphin,
            67 => Particle::CampfireCosySmoke,
            68 => Particle::CampfireSignalSmoke,
            69 => Particle::DrippingHoney,
            70 => Particle::FallingHoney,
            71 => Particle::LandingHoney,
            72 => Particle::FallingNectar,
            73 => Particle::FallingSporeBlossom,
            74 => Particle::Ash,
            75 => Particle::CrimsonSpore,
            76 => Particle::WarpedSpore,
            77 => Particle::SporeBlossomAir,
            78 => Particle::DrippingObsidianTear,
            79 => Particle::FallingObsidianTear,
            80 => Particle::LandingObsidianTear,
            81 => Particle::ReversePortal,
            82 => Particle::WhiteAsh,
            83 => Particle::SmallFlame,
            84 => Particle::Snowflake,
            85 => Particle::DrippingDripstoneLava,
            86 => Particle::FallingDripstoneLava,
            87 => Particle::DrippingDripstoneWater,
            88 => Particle::FallingDripstoneWater,
            89 => Particle::GlowSquidInk,
            90 => Particle::Glow,
            91 => Particle::WaxOn,
            92 => Particle::WaxOff,
            93 => Particle::ElectricSpark,
            94 => Particle::Scrape,
            95 => Particle::Shriek {
                delay: VarInt::decode(r)?.0,
            },
            id => bail!("invalid particle ID of {id}"),
        })
    }
}

impl Encode for ParticleS2c<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        VarInt(self.particle.id()).encode(&mut w)?;
        self.long_distance.encode(&mut w)?;
        self.position.encode(&mut w)?;
        self.offset.encode(&mut w)?;
        self.max_speed.encode(&mut w)?;
        self.count.encode(&mut w)?;

        self.particle.as_ref().encode(w)
    }
}

impl<'a> Decode<'a> for ParticleS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let particle_id = VarInt::decode(r)?.0;
        let long_distance = bool::decode(r)?;
        let position = <[f64; 3]>::decode(r)?;
        let offset = <[f32; 3]>::decode(r)?;
        let max_speed = f32::decode(r)?;
        let particle_count = i32::decode(r)?;

        Ok(Self {
            particle: Cow::Owned(Particle::decode_with_id(particle_id, r)?),
            long_distance,
            position,
            offset,
            max_speed,
            count: particle_count,
        })
    }
}

/// Encodes the particle without an ID.
impl Encode for Particle {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            Particle::Block(block_state) => block_state.encode(w),
            Particle::BlockMarker(block_state) => block_state.encode(w),
            Particle::Dust { rgb, scale } => {
                rgb.encode(&mut w)?;
                scale.encode(w)
            }
            Particle::DustColorTransition {
                from_rgb,
                scale,
                to_rgb,
            } => {
                from_rgb.encode(&mut w)?;
                scale.encode(&mut w)?;
                to_rgb.encode(w)
            }
            Particle::FallingDust(block_state) => block_state.encode(w),
            Particle::SculkCharge { roll } => roll.encode(w),
            Particle::Item(stack) => stack.encode(w),
            Particle::VibrationBlock { block_pos, ticks } => {
                "block".encode(&mut w)?;
                block_pos.encode(&mut w)?;
                VarInt(*ticks).encode(w)
            }
            Particle::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                "entity".encode(&mut w)?;
                VarInt(*entity_id).encode(&mut w)?;
                entity_eye_height.encode(&mut w)?;
                VarInt(*ticks).encode(w)
            }
            Particle::Shriek { delay } => VarInt(*delay).encode(w),
            _ => Ok(()),
        }
    }
}
