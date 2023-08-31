use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use valence_generated::block::BlockState;
use valence_math::{DVec3, Vec3};

use crate::{BlockPos, Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Packet)]
pub struct ParticleS2c<'a> {
    pub particle: Cow<'a, Particle>,
    pub long_distance: bool,
    pub position: DVec3,
    pub offset: Vec3,
    pub max_speed: f32,
    pub count: i32,
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
        let position = Decode::decode(r)?;
        let offset = Decode::decode(r)?;
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
        rgb: Vec3,
        scale: f32,
    },
    DustColorTransition {
        from_rgb: Vec3,
        scale: f32,
        to_rgb: Vec3,
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
    CherryLeaves,
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
    Item(ItemStack),
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
    EggCrack,
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
            Particle::CherryLeaves => 29,
            Particle::SculkSoul => 30,
            Particle::SculkCharge { .. } => 31,
            Particle::SculkChargePop => 32,
            Particle::SoulFireFlame => 33,
            Particle::Soul => 34,
            Particle::Flash => 35,
            Particle::HappyVillager => 36,
            Particle::Composter => 37,
            Particle::Heart => 38,
            Particle::InstantEffect => 39,
            Particle::Item { .. } => 40,
            Particle::VibrationBlock { .. } => 41,
            Particle::VibrationEntity { .. } => 41,
            Particle::ItemSlime => 42,
            Particle::ItemSnowball => 43,
            Particle::LargeSmoke => 44,
            Particle::Lava => 45,
            Particle::Mycelium => 46,
            Particle::Note => 47,
            Particle::Poof => 48,
            Particle::Portal => 49,
            Particle::Rain => 50,
            Particle::Smoke => 51,
            Particle::Sneeze => 52,
            Particle::Spit => 53,
            Particle::SquidInk => 54,
            Particle::SweepAttack => 55,
            Particle::TotemOfUndying => 56,
            Particle::Underwater => 57,
            Particle::Splash => 58,
            Particle::Witch => 59,
            Particle::BubblePop => 60,
            Particle::CurrentDown => 61,
            Particle::BubbleColumnUp => 62,
            Particle::Nautilus => 63,
            Particle::Dolphin => 64,
            Particle::CampfireCosySmoke => 65,
            Particle::CampfireSignalSmoke => 66,
            Particle::DrippingHoney => 67,
            Particle::FallingHoney => 68,
            Particle::LandingHoney => 69,
            Particle::FallingNectar => 70,
            Particle::FallingSporeBlossom => 71,
            Particle::Ash => 72,
            Particle::CrimsonSpore => 73,
            Particle::WarpedSpore => 74,
            Particle::SporeBlossomAir => 75,
            Particle::DrippingObsidianTear => 76,
            Particle::FallingObsidianTear => 77,
            Particle::LandingObsidianTear => 78,
            Particle::ReversePortal => 79,
            Particle::WhiteAsh => 80,
            Particle::SmallFlame => 81,
            Particle::Snowflake => 82,
            Particle::DrippingDripstoneLava => 83,
            Particle::FallingDripstoneLava => 84,
            Particle::DrippingDripstoneWater => 85,
            Particle::FallingDripstoneWater => 86,
            Particle::GlowSquidInk => 87,
            Particle::Glow => 88,
            Particle::WaxOn => 89,
            Particle::WaxOff => 80,
            Particle::ElectricSpark => 91,
            Particle::Scrape => 92,
            Particle::Shriek { .. } => 93,
            Particle::EggCrack => 94,
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
                rgb: Decode::decode(r)?,
                scale: Decode::decode(r)?,
            },
            15 => Particle::DustColorTransition {
                from_rgb: Decode::decode(r)?,
                scale: Decode::decode(r)?,
                to_rgb: Decode::decode(r)?,
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
            29 => Particle::CherryLeaves,
            30 => Particle::SculkSoul,
            31 => Particle::SculkCharge {
                roll: f32::decode(r)?,
            },
            32 => Particle::SculkChargePop,
            33 => Particle::SoulFireFlame,
            34 => Particle::Soul,
            35 => Particle::Flash,
            36 => Particle::HappyVillager,
            37 => Particle::Composter,
            38 => Particle::Heart,
            39 => Particle::InstantEffect,
            40 => Particle::Item(Decode::decode(r)?),
            41 => match <&str>::decode(r)? {
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
            42 => Particle::ItemSlime,
            43 => Particle::ItemSnowball,
            44 => Particle::LargeSmoke,
            45 => Particle::Lava,
            46 => Particle::Mycelium,
            47 => Particle::Note,
            48 => Particle::Poof,
            49 => Particle::Portal,
            50 => Particle::Rain,
            51 => Particle::Smoke,
            52 => Particle::Sneeze,
            53 => Particle::Spit,
            54 => Particle::SquidInk,
            55 => Particle::SweepAttack,
            56 => Particle::TotemOfUndying,
            57 => Particle::Underwater,
            58 => Particle::Splash,
            59 => Particle::Witch,
            60 => Particle::BubblePop,
            61 => Particle::CurrentDown,
            62 => Particle::BubbleColumnUp,
            63 => Particle::Nautilus,
            64 => Particle::Dolphin,
            65 => Particle::CampfireCosySmoke,
            66 => Particle::CampfireSignalSmoke,
            67 => Particle::DrippingHoney,
            68 => Particle::FallingHoney,
            69 => Particle::LandingHoney,
            70 => Particle::FallingNectar,
            71 => Particle::FallingSporeBlossom,
            72 => Particle::Ash,
            73 => Particle::CrimsonSpore,
            74 => Particle::WarpedSpore,
            75 => Particle::SporeBlossomAir,
            76 => Particle::DrippingObsidianTear,
            77 => Particle::FallingObsidianTear,
            78 => Particle::LandingObsidianTear,
            79 => Particle::ReversePortal,
            80 => Particle::WhiteAsh,
            81 => Particle::SmallFlame,
            82 => Particle::Snowflake,
            83 => Particle::DrippingDripstoneLava,
            84 => Particle::FallingDripstoneLava,
            85 => Particle::DrippingDripstoneWater,
            86 => Particle::FallingDripstoneWater,
            87 => Particle::GlowSquidInk,
            88 => Particle::Glow,
            89 => Particle::WaxOn,
            90 => Particle::WaxOff,
            91 => Particle::ElectricSpark,
            92 => Particle::Scrape,
            93 => Particle::Shriek {
                delay: VarInt::decode(r)?.0,
            },
            94 => Particle::EggCrack,
            id => bail!("invalid particle ID of {id}"),
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
