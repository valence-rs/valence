use std::io::Write;

use anyhow::Context;
use vek::{Rgb, Vec3};

use crate::block::{BlockPos, BlockState};
use crate::protocol::{Decode, Encode, VarInt};

#[derive(Clone, Debug)]
pub struct ParticleS2c {
    pub particle_type: Particle,
    pub long_distance: bool,
    pub position: Vec3<f64>,
    pub offset: Vec3<f32>,
    pub max_speed: f32,
    pub particle_count: u32,
}

#[derive(Clone, Debug)]
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
        rgb: Rgb<f32>,
        scale: f32,
    },
    DustColorTransition {
        from_rgb: Rgb<f32>,
        scale: f32,
        to_rgb: Rgb<f32>,
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
    VibrationBlock {
        // The 'Block' variant of the 'Vibration' particle
        block_pos: BlockPos,
        ticks: i32,
    },
    VibrationEntity {
        // The 'Entity' variant of the 'Vibration' particle
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
}

impl Particle {
    fn id(&self) -> i32 {
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
            Particle::SculkSoul => 29,
            Particle::SculkCharge { .. } => 30,
            Particle::SculkChargePop => 31,
            Particle::SoulFireFlame => 32,
            Particle::Soul => 33,
            Particle::Flash => 34,
            Particle::HappyVillager => 35,
            Particle::Composter => 36,
            Particle::Heart => 37,
            Particle::InstantEffect => 38,
            Particle::Item(_) => 39,
            Particle::VibrationBlock { .. } => 40,
            Particle::VibrationEntity { .. } => 40,
            Particle::ItemSlime => 41,
            Particle::ItemSnowball => 42,
            Particle::LargeSmoke => 43,
            Particle::Lava => 44,
            Particle::Mycelium => 45,
            Particle::Note => 46,
            Particle::Poof => 47,
            Particle::Portal => 48,
            Particle::Rain => 49,
            Particle::Smoke => 50,
            Particle::Sneeze => 51,
            Particle::Spit => 52,
            Particle::SquidInk => 53,
            Particle::SweepAttack => 54,
            Particle::TotemOfUndying => 55,
            Particle::Underwater => 56,
            Particle::Splash => 57,
            Particle::Witch => 58,
            Particle::BubblePop => 59,
            Particle::CurrentDown => 60,
            Particle::BubbleColumnUp => 61,
            Particle::Nautilus => 62,
            Particle::Dolphin => 63,
            Particle::CampfireCosySmoke => 64,
            Particle::CampfireSignalSmoke => 65,
            Particle::DrippingHoney => 66,
            Particle::FallingHoney => 67,
            Particle::LandingHoney => 68,
            Particle::FallingNectar => 69,
            Particle::FallingSporeBlossom => 70,
            Particle::Ash => 71,
            Particle::CrimsonSpore => 72,
            Particle::WarpedSpore => 73,
            Particle::SporeBlossomAir => 74,
            Particle::DrippingObsidianTear => 75,
            Particle::FallingObsidianTear => 76,
            Particle::LandingObsidianTear => 77,
            Particle::ReversePortal => 78,
            Particle::WhiteAsh => 79,
            Particle::SmallFlame => 80,
            Particle::Snowflake => 81,
            Particle::DrippingDripstoneLava => 82,
            Particle::FallingDripstoneLava => 83,
            Particle::DrippingDripstoneWater => 84,
            Particle::FallingDripstoneWater => 85,
            Particle::GlowSquidInk => 86,
            Particle::Glow => 87,
            Particle::WaxOn => 88,
            Particle::WaxOff => 89,
            Particle::ElectricSpark => 90,
            Particle::Scrape => 91,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Particle::AmbientEntityEffect => "ambient_entity_effect",
            Particle::AngryVillager => "angry_villager",
            Particle::Block(_) => "block",
            Particle::BlockMarker(_) => "block_marker",
            Particle::Bubble => "bubble",
            Particle::Cloud => "cloud",
            Particle::Crit => "crit",
            Particle::DamageIndicator => "damage_indicator",
            Particle::DragonBreath => "dragon_breath",
            Particle::DrippingLava => "dripping_lava",
            Particle::FallingLava => "falling_lava",
            Particle::LandingLava => "landing_lava",
            Particle::DrippingWater => "dripping_water",
            Particle::FallingWater => "falling_water",
            Particle::Dust { .. } => "dust",
            Particle::DustColorTransition { .. } => "dust_color_transition",
            Particle::Effect => "effect",
            Particle::ElderGuardian => "elder_guardian",
            Particle::EnchantedHit => "enchanted_hit",
            Particle::Enchant => "enchant",
            Particle::EndRod => "end_rod",
            Particle::EntityEffect => "entity_effect",
            Particle::ExplosionEmitter => "explosion_emitter",
            Particle::Explosion => "explosion",
            Particle::SonicBoom => "sonic_boom",
            Particle::FallingDust(_) => "falling_dust",
            Particle::Firework => "firework",
            Particle::Fishing => "fishing",
            Particle::Flame => "flame",
            Particle::SculkSoul => "sculk_soul",
            Particle::SculkCharge { .. } => "sculk_charge",
            Particle::SculkChargePop => "sculk_charge_pop",
            Particle::SoulFireFlame => "soul_fire_flame",
            Particle::Soul => "soul",
            Particle::Flash => "flash",
            Particle::HappyVillager => "happy_villager",
            Particle::Composter => "composter",
            Particle::Heart => "heart",
            Particle::InstantEffect => "instant_effect",
            Particle::Item(_) => "item",
            Particle::VibrationBlock { .. } => "vibration",
            Particle::VibrationEntity { .. } => "vibration",
            Particle::ItemSlime => "item_slime",
            Particle::ItemSnowball => "item_snowball",
            Particle::LargeSmoke => "large_smoke",
            Particle::Lava => "lava",
            Particle::Mycelium => "mycelium",
            Particle::Note => "note",
            Particle::Poof => "poof",
            Particle::Portal => "portal",
            Particle::Rain => "rain",
            Particle::Smoke => "smoke",
            Particle::Sneeze => "sneeze",
            Particle::Spit => "spit",
            Particle::SquidInk => "squid_ink",
            Particle::SweepAttack => "sweep_attack",
            Particle::TotemOfUndying => "totem_of_undying",
            Particle::Underwater => "underwater",
            Particle::Splash => "splash",
            Particle::Witch => "witch",
            Particle::BubblePop => "bubble_pop",
            Particle::CurrentDown => "current_down",
            Particle::BubbleColumnUp => "bubble_column_up",
            Particle::Nautilus => "nautilus",
            Particle::Dolphin => "dolphin",
            Particle::CampfireCosySmoke => "campfire_cosy_smoke",
            Particle::CampfireSignalSmoke => "campfire_signal_smoke",
            Particle::DrippingHoney => "dripping_honey",
            Particle::FallingHoney => "falling_honey",
            Particle::LandingHoney => "landing_honey",
            Particle::FallingNectar => "falling_nectar",
            Particle::FallingSporeBlossom => "falling_spore_blossom",
            Particle::Ash => "ash",
            Particle::CrimsonSpore => "crimson_spore",
            Particle::WarpedSpore => "warped_spore",
            Particle::SporeBlossomAir => "spore_blossom_air",
            Particle::DrippingObsidianTear => "dripping_obsidian_tear",
            Particle::FallingObsidianTear => "falling_obsidian_tear",
            Particle::LandingObsidianTear => "landing_obsidian_tear",
            Particle::ReversePortal => "reverse_portal", // Particle does not have name
            Particle::WhiteAsh => "white_ash",
            Particle::SmallFlame => "small_flame",
            Particle::Snowflake => "snowflake",
            Particle::DrippingDripstoneLava => "dripping_dripstone_lava",
            Particle::FallingDripstoneLava => "falling_dripstone_lava",
            Particle::DrippingDripstoneWater => "dripping_dripstone_water",
            Particle::FallingDripstoneWater => "falling_dripstone_water",
            Particle::GlowSquidInk => "glow_squid_ink",
            Particle::Glow => "glow",
            Particle::WaxOn => "wax_on",
            Particle::WaxOff => "wax_off",
            Particle::ElectricSpark => "electric_spark",
            Particle::Scrape => "scrape",
        }
    }
}

impl Encode for ParticleS2c {
    fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
        let particle_id: VarInt = VarInt(self.particle_type.id());
        Encode::encode(&particle_id, _w)
            .context("failed to write particle id from struct `ParticleS2c`")?;
        Encode::encode(&self.long_distance, _w)
            .context("failed to write field `long_distance` from struct `ParticleS2c`")?;
        Encode::encode(&self.position, _w)
            .context("failed to write field `position` from struct `ParticleS2c`")?;
        Encode::encode(&self.offset, _w)
            .context("failed to write field `offset` from struct `ParticleS2c`")?;
        Encode::encode(&self.max_speed, _w)
            .context("failed to write field `max_speed` from struct `ParticleS2c`")?;
        Encode::encode(&self.particle_count, _w)
            .context("failed to write field `particle_count` from struct `ParticleS2c`")?;
        Encode::encode(&self.particle_type, _w)
            .context("failed to write field `particle_type` from struct `ParticleS2c`")?;
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

impl Decode for ParticleS2c {
    fn decode(_r: &mut &[u8]) -> anyhow::Result<Self> {
        todo!("Is this even necessary?");
        // let particle_id: VarInt = Decode::decode(_r).context(concat!("failed
        // to read field `", stringify!(particle_id), "` from struct `",
        // stringify!(Particle), "`"))?;
    }
}

impl Encode for Particle {
    fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
        match self {
            Particle::Block(block_state) => {
                Encode::encode(block_state, _w)
                    .context("failed to write field `block_state` from struct `Particle::Block`")?;
            }
            Particle::BlockMarker(block_state) => {
                Encode::encode(block_state, _w).context(
                    "failed to write field `block_state` from struct `Particle::BlockMarker`",
                )?;
            }
            Particle::Dust { rgb, scale } => {
                Encode::encode(rgb, _w)
                    .context("failed to write field `rgb` from struct `Particle::Dust`")?;
                Encode::encode(scale, _w)
                    .context("failed to write field `scale` from struct `Particle::Dust`")?;
            }
            Particle::DustColorTransition {
                from_rgb,
                scale,
                to_rgb,
            } => {
                Encode::encode(from_rgb, _w).context(
                    "failed to write field `from_rgb` from struct `Particle::DustColorTransition`",
                )?;
                Encode::encode(scale, _w).context(
                    "failed to write field `scale` from struct `Particle::DustColorTransition`",
                )?;
                Encode::encode(to_rgb, _w).context(
                    "failed to write field `to_rgb` from struct `Particle::DustColorTransition`",
                )?;
            }
            Particle::FallingDust(block_state) => {
                Encode::encode(block_state, _w).context(
                    "failed to write field `block_state` from struct `Particle::FallingDust`",
                )?;
            }
            Particle::SculkCharge { roll } => {
                Encode::encode(roll, _w).context(
                    "failed to write field `block_state` from struct `Particle::FallingDust`",
                )?;
            }
            Particle::Item(_) => todo!("Item particle not yet implemented"),
            Particle::VibrationBlock { block_pos, ticks } => {
                Encode::encode("block", _w).context(
                    "failed to write field `position_source_type` from struct \
                     `Particle::VibrationBlock`",
                )?;
                Encode::encode(block_pos, _w).context(
                    "failed to write field `block_pos` from struct `Particle::VibrationBlock`",
                )?;
                Encode::encode(&VarInt(*ticks), _w).context(
                    "failed to write field `ticks` from struct `Particle::VibrationBlock`",
                )?;
            }
            Particle::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                Encode::encode("entity", _w).context(
                    "failed to write field `position_source_type` from struct \
                     `Particle::VibrationEntity`",
                )?;
                Encode::encode(&VarInt(*entity_id), _w).context(
                    "failed to write field `entity_id` from struct `Particle::VibrationEntity`",
                )?;
                Encode::encode(entity_eye_height, _w).context(
                    "failed to write field `entity_eye_height` from struct \
                     `Particle::VibrationEntity`",
                )?;
                Encode::encode(&VarInt(*ticks), _w).context(
                    "failed to write field `ticks` from struct `Particle::VibrationEntity`",
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let id_len = VarInt(self.id()).encoded_len();
        let data_len = match self {
            Particle::Block(block_state) => block_state.encoded_len(),
            Particle::BlockMarker(block_state) => block_state.encoded_len(),
            Particle::Dust { .. } => 4 * 4,
            Particle::DustColorTransition { .. } => 7 * 4,
            Particle::FallingDust(block_state) => block_state.encoded_len(),
            Particle::SculkCharge { .. } => 4,
            Particle::Item(_) => todo!("Item particle not yet implemented"),
            Particle::VibrationBlock { block_pos, ticks } => {
                "block".encoded_len() + block_pos.encoded_len() + VarInt(*ticks).encoded_len()
            }
            Particle::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                "entity".encoded_len()
                    + VarInt(*entity_id).encoded_len()
                    + entity_eye_height.encoded_len()
                    + VarInt(*ticks).encoded_len()
            }
            _ => 0,
        };
        id_len + data_len
    }
}
