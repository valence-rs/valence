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
    VibrationBlock {
        // The 'Block' variant of the 'Vibration' particle
        block_pos: BlockPos,
        ticks: VarInt,
    },
    VibrationEntity {
        // The 'Entity' variant of the 'Vibration' particle
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
            ParticleType::SculkCharge { .. } => 30,
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

    pub fn name(&self) -> &str {
        match self {
            ParticleType::AmbientEntityEffect => "ambient_entity_effect",
            ParticleType::AngryVillager => "angry_villager",
            ParticleType::Block(_) => "block",
            ParticleType::BlockMarker(_) => "block_marker",
            ParticleType::Bubble => "bubble",
            ParticleType::Cloud => "cloud",
            ParticleType::Crit => "crit",
            ParticleType::DamageIndicator => "damage_indicator",
            ParticleType::DragonBreath => "dragon_breath",
            ParticleType::DrippingLava => "dripping_lava",
            ParticleType::FallingLava => "falling_lava",
            ParticleType::LandingLava => "landing_lava",
            ParticleType::DrippingWater => "dripping_water",
            ParticleType::FallingWater => "falling_water",
            ParticleType::Dust { .. } => "dust",
            ParticleType::DustColorTransition { .. } => "dust_color_transition",
            ParticleType::Effect => "effect",
            ParticleType::ElderGuardian => "elder_guardian",
            ParticleType::EnchantedHit => "enchanted_hit",
            ParticleType::Enchant => "enchant",
            ParticleType::EndRod => "end_rod",
            ParticleType::EntityEffect => "entity_effect",
            ParticleType::ExplosionEmitter => "explosion_emitter",
            ParticleType::Explosion => "explosion",
            ParticleType::SonicBoom => "sonic_boom",
            ParticleType::FallingDust(_) => "falling_dust",
            ParticleType::Firework => "firework",
            ParticleType::Fishing => "fishing",
            ParticleType::Flame => "flame",
            ParticleType::SculkSoul => "sculk_soul",
            ParticleType::SculkCharge { .. } => "sculk_charge",
            ParticleType::SculkChargePop => "sculk_charge_pop",
            ParticleType::SoulFireFlame => "soul_fire_flame",
            ParticleType::Soul => "soul",
            ParticleType::Flash => "flash",
            ParticleType::HappyVillager => "happy_villager",
            ParticleType::Composter => "composter",
            ParticleType::Heart => "heart",
            ParticleType::InstantEffect => "instant_effect",
            ParticleType::Item(_) => "item",
            ParticleType::VibrationBlock { .. } => "vibration",
            ParticleType::VibrationEntity { .. } => "vibration",
            ParticleType::ItemSlime => "item_slime",
            ParticleType::ItemSnowball => "item_snowball",
            ParticleType::LargeSmoke => "large_smoke",
            ParticleType::Lava => "lava",
            ParticleType::Mycelium => "mycelium",
            ParticleType::Note => "note",
            ParticleType::Poof => "poof",
            ParticleType::Portal => "portal",
            ParticleType::Rain => "rain",
            ParticleType::Smoke => "smoke",
            ParticleType::Sneeze => "sneeze",
            ParticleType::Spit => "spit",
            ParticleType::SquidInk => "squid_ink",
            ParticleType::SweepAttack => "sweep_attack",
            ParticleType::TotemOfUndying => "totem_of_undying",
            ParticleType::Underwater => "underwater",
            ParticleType::Splash => "splash",
            ParticleType::Witch => "witch",
            ParticleType::BubblePop => "bubble_pop",
            ParticleType::CurrentDown => "current_down",
            ParticleType::BubbleColumnUp => "bubble_column_up",
            ParticleType::Nautilus => "nautilus",
            ParticleType::Dolphin => "dolphin",
            ParticleType::CampfireCosySmoke => "campfire_cosy_smoke",
            ParticleType::CampfireSignalSmoke => "campfire_signal_smoke",
            ParticleType::DrippingHoney => "dripping_honey",
            ParticleType::FallingHoney => "falling_honey",
            ParticleType::LandingHoney => "landing_honey",
            ParticleType::FallingNectar => "falling_nectar",
            ParticleType::FallingSporeBlossom => "falling_spore_blossom",
            ParticleType::Ash => "ash",
            ParticleType::CrimsonSpore => "crimson_spore",
            ParticleType::WarpedSpore => "warped_spore",
            ParticleType::SporeBlossomAir => "spore_blossom_air",
            ParticleType::DrippingObsidianTear => "dripping_obsidian_tear",
            ParticleType::FallingObsidianTear => "falling_obsidian_tear",
            ParticleType::LandingObsidianTear => "landing_obsidian_tear",
            ParticleType::ReversePortal => "reverse_portal", // Particle does not have name
            ParticleType::WhiteAsh => "white_ash",
            ParticleType::SmallFlame => "small_flame",
            ParticleType::Snowflake => "snowflake",
            ParticleType::DrippingDripstoneLava => "dripping_dripstone_lava",
            ParticleType::FallingDripstoneLava => "falling_dripstone_lava",
            ParticleType::DrippingDripstoneWater => "dripping_dripstone_water",
            ParticleType::FallingDripstoneWater => "falling_dripstone_water",
            ParticleType::GlowSquidInk => "glow_squid_ink",
            ParticleType::Glow => "glow",
            ParticleType::WaxOn => "wax_on",
            ParticleType::WaxOff => "wax_off",
            ParticleType::ElectricSpark => "electric_spark",
            ParticleType::Scrape => "scrape",
        }
    }
}

impl Encode for Particle {
    fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
        let particle_id: VarInt = VarInt(self.particle_type.id());
        Encode::encode(&particle_id, _w)
            .context("failed to write particle id from struct `Particle`")?;
        Encode::encode(&self.long_distance, _w)
            .context("failed to write field `long_distance` from struct `Particle`")?;
        Encode::encode(&self.position, _w)
            .context("failed to write field `position` from struct `Particle`")?;
        Encode::encode(&self.offset, _w)
            .context("failed to write field `offset` from struct `Particle`")?;
        Encode::encode(&self.max_speed, _w)
            .context("failed to write field `max_speed` from struct `Particle`")?;
        Encode::encode(&self.particle_count, _w)
            .context("failed to write field `particle_count` from struct `Particle`")?;
        Encode::encode(&self.particle_type, _w)
            .context("failed to write field `particle_type` from struct `Particle`")?;
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
                Encode::encode(block_state, _w).context(
                    "failed to write field `block_state` from struct `ParticleType::Block`",
                )?;
            }
            ParticleType::BlockMarker(block_state) => {
                Encode::encode(block_state, _w).context(
                    "failed to write field `block_state` from struct `ParticleType::BlockMarker`",
                )?;
            }
            ParticleType::Dust { rgb, scale } => {
                Encode::encode(rgb, _w)
                    .context("failed to write field `rgb` from struct `ParticleType::Dust`")?;
                Encode::encode(scale, _w)
                    .context("failed to write field `scale` from struct `ParticleType::Dust`")?;
            }
            ParticleType::DustColorTransition {
                from_rgb,
                scale,
                to_rgb,
            } => {
                Encode::encode(from_rgb, _w).context(
                    "failed to write field `from_rgb` from struct \
                     `ParticleType::DustColorTransition`",
                )?;
                Encode::encode(scale, _w).context(
                    "failed to write field `scale` from struct `ParticleType::DustColorTransition`",
                )?;
                Encode::encode(to_rgb, _w).context(
                    "failed to write field `to_rgb` from struct \
                     `ParticleType::DustColorTransition`",
                )?;
            }
            ParticleType::FallingDust(block_state) => {
                Encode::encode(block_state, _w).context(
                    "failed to write field `block_state` from struct `ParticleType::FallingDust`",
                )?;
            }
            ParticleType::SculkCharge { roll } => {
                Encode::encode(roll, _w).context(
                    "failed to write field `block_state` from struct `ParticleType::FallingDust`",
                )?;
            }
            ParticleType::Item(_) => todo!("Item particle not yet implemented"),
            ParticleType::VibrationBlock { block_pos, ticks } => {
                Encode::encode(&"block".to_string(), _w).context(
                    "failed to write field `position_source_type` from struct \
                     `ParticleType::VibrationBlock`",
                )?;
                Encode::encode(block_pos, _w).context(
                    "failed to write field `block_pos` from struct `ParticleType::VibrationBlock`",
                )?;
                Encode::encode(ticks, _w).context(
                    "failed to write field `ticks` from struct `ParticleType::VibrationBlock`",
                )?;
            }
            ParticleType::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                Encode::encode(&"entity".to_string(), _w).context(
                    "failed to write field `position_source_type` from struct \
                     `ParticleType::VibrationEntity`",
                )?;
                Encode::encode(entity_id, _w).context(
                    "failed to write field `entity_id` from struct `ParticleType::VibrationEntity`",
                )?;
                Encode::encode(entity_eye_height, _w).context(
                    "failed to write field `entity_eye_height` from struct \
                     `ParticleType::VibrationEntity`",
                )?;
                Encode::encode(ticks, _w).context(
                    "failed to write field `ticks` from struct `ParticleType::VibrationEntity`",
                )?;
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
            ParticleType::VibrationBlock { block_pos, ticks } => {
                "block".encoded_len() + block_pos.encoded_len() + ticks.encoded_len()
            }
            ParticleType::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                "entity".encoded_len()
                    + entity_id.encoded_len()
                    + entity_eye_height.encoded_len()
                    + ticks.encoded_len()
            }
            _ => 0,
        };
        id_len + data_len
    }
}
