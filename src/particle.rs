use std::io::Write;

use anyhow::bail;
use vek::{Rgb, Vec3};

use crate::block::{BlockPos, BlockState};
use crate::item::ItemStack;
use crate::protocol::{Decode, Encode, Slot, VarInt};

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
    Item(Option<ItemStack>),
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
        Encode::encode(&particle_id, _w)?;
        Encode::encode(&self.long_distance, _w)?;
        Encode::encode(&self.position, _w)?;
        Encode::encode(&self.offset, _w)?;
        Encode::encode(&self.max_speed, _w)?;
        Encode::encode(&self.particle_count, _w)?;
        Encode::encode(&self.particle_type, _w)?;
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

impl Encode for Particle {
    fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
        match self {
            Particle::Block(block_state) => {
                Encode::encode(block_state, _w)?;
            }
            Particle::BlockMarker(block_state) => {
                Encode::encode(block_state, _w)?;
            }
            Particle::Dust { rgb, scale } => {
                Encode::encode(rgb, _w)?;
                Encode::encode(scale, _w)?;
            }
            Particle::DustColorTransition {
                from_rgb,
                scale,
                to_rgb,
            } => {
                Encode::encode(from_rgb, _w)?;
                Encode::encode(scale, _w)?;
                Encode::encode(to_rgb, _w)?;
            }
            Particle::FallingDust(block_state) => {
                Encode::encode(block_state, _w)?;
            }
            Particle::SculkCharge { roll } => {
                Encode::encode(roll, _w)?;
            }
            Particle::Item(stack) => {
                let slot: &Slot = stack;
                Encode::encode(slot, _w)?;
            }
            Particle::VibrationBlock { block_pos, ticks } => {
                Encode::encode("block", _w)?;
                Encode::encode(block_pos, _w)?;
                Encode::encode(&VarInt(*ticks), _w)?;
            }
            Particle::VibrationEntity {
                entity_id,
                entity_eye_height,
                ticks,
            } => {
                Encode::encode("entity", _w)?;
                Encode::encode(&VarInt(*entity_id), _w)?;
                Encode::encode(entity_eye_height, _w)?;
                Encode::encode(&VarInt(*ticks), _w)?;
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
            Particle::Item(stack) => {
                let slot: &Slot = stack;
                slot.encoded_len()
            }
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

impl Decode for ParticleS2c {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let particle_id: VarInt = Decode::decode(r)?;
        let long_distance: bool = Decode::decode(r)?;
        let position: Vec3<f64> = Decode::decode(r)?;
        let offset: Vec3<f32> = Decode::decode(r)?;
        let max_speed: f32 = Decode::decode(r)?;
        let particle_count: u32 = Decode::decode(r)?;
        let particle = match particle_id.0 {
            0 => Particle::AmbientEntityEffect,
            1 => Particle::AngryVillager,
            2 => {
                let block_state: BlockState = Decode::decode(r)?;
                Particle::Block(block_state)
            }
            3 => {
                let block_state: BlockState = Decode::decode(r)?;
                Particle::BlockMarker(block_state)
            }
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
            14 => {
                let rgb: Rgb<f32> = Decode::decode(r)?;
                let scale: f32 = Decode::decode(r)?;
                Particle::Dust { rgb, scale }
            }
            15 => {
                let from_rgb: Rgb<f32> = Decode::decode(r)?;
                let scale: f32 = Decode::decode(r)?;
                let to_rgb: Rgb<f32> = Decode::decode(r)?;
                Particle::DustColorTransition {
                    from_rgb,
                    scale,
                    to_rgb,
                }
            }
            16 => Particle::Effect,
            17 => Particle::ElderGuardian,
            18 => Particle::EnchantedHit,
            19 => Particle::Enchant,
            20 => Particle::EndRod,
            21 => Particle::EntityEffect,
            22 => Particle::ExplosionEmitter,
            23 => Particle::Explosion,
            24 => Particle::SonicBoom,
            25 => {
                let block_state: BlockState = Decode::decode(r)?;
                Particle::FallingDust(block_state)
            }
            26 => Particle::Firework,
            27 => Particle::Fishing,
            28 => Particle::Flame,
            29 => Particle::SculkSoul,
            30 => {
                let roll: f32 = Decode::decode(r)?;
                Particle::SculkCharge { roll }
            }
            31 => Particle::SculkChargePop,
            32 => Particle::SoulFireFlame,
            33 => Particle::Soul,
            34 => Particle::Flash,
            35 => Particle::HappyVillager,
            36 => Particle::Composter,
            37 => Particle::Heart,
            38 => Particle::InstantEffect,
            39 => {
                let slot: Slot = Decode::decode(r)?;
                Particle::Item(slot)
            }
            40 => {
                let position_source_type: String = Decode::decode(r)?;
                match position_source_type.as_str() {
                    "block" => {
                        let block_pos: BlockPos = Decode::decode(r)?;
                        let ticks: VarInt = Decode::decode(r)?;
                        Particle::VibrationBlock {
                            block_pos,
                            ticks: ticks.0,
                        }
                    }
                    "entity" => {
                        let entity_id: VarInt = Decode::decode(r)?;
                        let entity_eye_height: f32 = Decode::decode(r)?;
                        let ticks: VarInt = Decode::decode(r)?;
                        Particle::VibrationEntity {
                            entity_id: entity_id.0,
                            entity_eye_height,
                            ticks: ticks.0,
                        }
                    }
                    invalid => {
                        bail!("invalid position_source_type {invalid}");
                    }
                }
            }
            41 => Particle::ItemSlime,
            42 => Particle::ItemSnowball,
            43 => Particle::LargeSmoke,
            44 => Particle::Lava,
            45 => Particle::Mycelium,
            46 => Particle::Note,
            47 => Particle::Poof,
            48 => Particle::Portal,
            49 => Particle::Rain,
            50 => Particle::Smoke,
            51 => Particle::Sneeze,
            52 => Particle::Spit,
            53 => Particle::SquidInk,
            54 => Particle::SweepAttack,
            55 => Particle::TotemOfUndying,
            56 => Particle::Underwater,
            57 => Particle::Splash,
            58 => Particle::Witch,
            59 => Particle::BubblePop,
            60 => Particle::CurrentDown,
            61 => Particle::BubbleColumnUp,
            62 => Particle::Nautilus,
            63 => Particle::Dolphin,
            64 => Particle::CampfireCosySmoke,
            65 => Particle::CampfireSignalSmoke,
            66 => Particle::DrippingHoney,
            67 => Particle::FallingHoney,
            68 => Particle::LandingHoney,
            69 => Particle::FallingNectar,
            70 => Particle::FallingSporeBlossom,
            71 => Particle::Ash,
            72 => Particle::CrimsonSpore,
            73 => Particle::WarpedSpore,
            74 => Particle::SporeBlossomAir,
            75 => Particle::DrippingObsidianTear,
            76 => Particle::FallingObsidianTear,
            77 => Particle::LandingObsidianTear,
            78 => Particle::ReversePortal,
            79 => Particle::WhiteAsh,
            80 => Particle::SmallFlame,
            81 => Particle::Snowflake,
            82 => Particle::DrippingDripstoneLava,
            83 => Particle::FallingDripstoneLava,
            84 => Particle::DrippingDripstoneWater,
            85 => Particle::FallingDripstoneWater,
            86 => Particle::GlowSquidInk,
            87 => Particle::Glow,
            88 => Particle::WaxOn,
            89 => Particle::WaxOff,
            90 => Particle::ElectricSpark,
            91 => Particle::Scrape,
            id => bail!("invalid particle ID {id}"),
        };
        let particle_packet: ParticleS2c = ParticleS2c {
            particle_type: particle,
            long_distance,
            position,
            offset,
            max_speed,
            particle_count,
        };
        Ok(particle_packet)
    }
}
