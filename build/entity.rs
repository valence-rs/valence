//! See: <https://wiki.vg/Entity_metadata> and <https://wiki.vg/Entity_statuses>

use std::collections::{BTreeMap, HashMap};

use anyhow::Context;
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::{ident, write_to_out_path};

struct Class {
    name: &'static str,
    inherit: Option<&'static Class>,
    fields: &'static [Field],
    events: &'static [Event],
}

impl Class {
    pub fn collect_fields(&self, fields: &mut Vec<&'static Field>) {
        if let Some(parent) = self.inherit {
            parent.collect_fields(fields);
        }
        fields.extend(self.fields);
    }

    pub fn collect_events(&self, events: &mut Vec<Event>) {
        if let Some(parent) = self.inherit {
            parent.collect_events(events);
        }
        events.extend(self.events);
    }
}

struct Field {
    name: &'static str,
    typ: Type,
}

macro_rules! def_events {
    (
        $(
            $variant:ident $(= $val:expr)?
        ),* $(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[allow(dead_code)]
        enum Event {
            $($variant $(= $val)*,)*
        }

        impl Event {
            pub fn snake_case_name(self) -> String {
                match self {
                    $(
                        Self::$variant => stringify!($variant).to_snake_case(),
                    )*
                }
            }
        }
    }
}

def_events! {
    // Entity events
    Jump = 1,
    Hurt,
    Death,
    StartAttacking,
    StopAttacking,
    TamingFailed,
    TamingSucceeded,
    ShakeWetness,
    UseItemComplete,
    EatGrass,
    OfferFlower,
    LoveHearts,
    VillagerAngry,
    VillagerHappy,
    WitchHatMagic,
    ZombieConverting,
    FireworksExplode,
    InLoveHearts,
    SquidAnimSynch,
    SilverfishMergeAnim,
    GuardianAttackSound,
    ReducedDebugInfo,
    FullDebugInfo,
    PermissionLevelAll,
    PermissionLevelModerators,
    PermissionLevelGamemasters,
    PermissionLevelAdmins,
    PermissionLevelOwners,
    AttackBlocked,
    ShieldDisabled,
    FishingRodReelIn,
    ArmorstandWobble,
    Thorned,
    StopOfferFlower,
    TalismanActivate,
    Drowned,
    Burned,
    DolphinLookingForTreasure,
    RavagerStunned,
    TrustingFailed,
    TrustingSucceeded,
    VillagerSweat,
    BadOmenTriggered,
    Poked,
    FoxEat,
    Teleport,
    MainhandBreak,
    OffhandBreak,
    HeadBreak,
    ChestBreak,
    LegsBreak,
    FeetBreak,
    HoneySlide,
    HoneyJump,
    SwapHands,
    CancelShakeWetness,
    Frozen,
    StartRam,
    EndRam,
    Poof,
    TendrilsShiver,
    SonicCharge,
    // Animations
    SwingMainArm,
    TakeDamage,
    LeaveBed,
    SwingOffhand,
    CriticalEffect,
    MagicCriticalEffect,
}

/// Each variant contains the default value for the field.
enum Type {
    BitFields(&'static [BitField]),
    Byte(u8),
    VarInt(i32),
    Float(f32),
    String(&'static str),
    Text,
    OptText(Option<&'static str>),
    Slot,
    Bool(bool),
    ArmorStandRotations(f32, f32, f32),
    BlockPos(i32, i32, i32),
    OptBlockPos(Option<(i32, i32, i32)>),
    Direction,
    OptUuid,
    BlockState,
    Nbt,
    Particle,
    VillagerData,
    /// Also known as OptVarInt
    OptEntityId,
    Pose,
    CatKind,
    FrogKind,
    OptGlobalPosition,
    PaintingKind,
    // ==== Specialized ==== //
    BoatKind,
    MainHand,
}

impl Type {
    pub fn default_expr(&self) -> TokenStream {
        match self {
            Type::BitFields(bfs) => {
                let mut default = 0;
                for bf in *bfs {
                    default = (bf.default as u8) << bf.offset;
                }
                quote! { #default }
            }
            Type::Byte(d) => quote! { #d },
            Type::VarInt(d) => quote! { VarInt(#d) },
            Type::Float(d) => quote! { #d },
            Type::String(d) => quote! { #d.into() },
            Type::Text => quote! { Default::default() },
            Type::OptText(d) => match d {
                Some(d) => quote! { Some(Box::new(Text::from(#d))) },
                None => quote! { None },
            },
            Type::Slot => quote! { () }, // TODO
            Type::Bool(d) => quote! { #d },
            Type::ArmorStandRotations(x, y, z) => {
                quote! { ArmorStandRotations::new(#x, #y, #z) }
            }
            Type::BlockPos(x, y, z) => quote! { BlockPos::new(#x, #y, #z) },
            Type::OptBlockPos(d) => match d {
                Some((x, y, z)) => quote! { Some(BlockPos::new(#x, #y, #z)) },
                None => quote! { None },
            },
            Type::Direction => quote! { Direction::Down },
            Type::OptUuid => quote! { None },
            Type::BlockState => quote! { BlockState::AIR },
            Type::Nbt => quote! { nbt::Blob::new() },
            Type::Particle => quote! { () }, // TODO
            Type::VillagerData => quote! { VillagerData::default() },
            Type::OptEntityId => quote! { None },
            Type::Pose => quote! { Pose::default() },
            Type::CatKind => quote! { CatKind::default() },
            Type::FrogKind => quote! { FrogKind::default() },
            Type::OptGlobalPosition => quote! { () }, // TODO
            Type::PaintingKind => quote! { PaintingKind::default() },
            Type::BoatKind => quote! { BoatKind::default() },
            Type::MainHand => quote! { MainHand::default() },
        }
    }

    pub fn type_id(&self) -> i32 {
        match self {
            Type::BitFields(_) => 0,
            Type::Byte(_) => 0,
            Type::VarInt(_) => 1,
            Type::Float(_) => 2,
            Type::String(_) => 3,
            Type::Text => 4,
            Type::OptText(_) => 5,
            Type::Slot => 6,
            Type::Bool(_) => 7,
            Type::ArmorStandRotations(_, _, _) => 8,
            Type::BlockPos(_, _, _) => 9,
            Type::OptBlockPos(_) => 10,
            Type::Direction => 11,
            Type::OptUuid => 12,
            Type::BlockState => 13,
            Type::Nbt => 14,
            Type::Particle => 15,
            Type::VillagerData => 16,
            Type::OptEntityId => 17,
            Type::Pose => 18,
            Type::CatKind => 19,
            Type::FrogKind => 20,
            Type::OptGlobalPosition => 21,
            Type::PaintingKind => 22,
            Type::BoatKind => 1,
            Type::MainHand => 0,
        }
    }
}

struct BitField {
    name: &'static str,
    offset: u8,
    default: bool,
}

const BASE_ENTITY: Class = Class {
    name: "base_entity",
    inherit: None,
    fields: &[
        Field {
            name: "base_entity_flags",
            typ: Type::BitFields(&[
                BitField {
                    name: "on_fire",
                    offset: 0,
                    default: false,
                },
                BitField {
                    name: "crouching",
                    offset: 1,
                    default: false,
                },
                BitField {
                    name: "sprinting",
                    offset: 3, // Skipping unused
                    default: false,
                },
                BitField {
                    name: "swimming",
                    offset: 4,
                    default: false,
                },
                BitField {
                    name: "invisible",
                    offset: 5,
                    default: false,
                },
                BitField {
                    name: "glowing",
                    offset: 6,
                    default: false,
                },
                BitField {
                    name: "elytra_flying",
                    offset: 7,
                    default: false,
                },
            ]),
        },
        Field {
            name: "air_ticks",
            typ: Type::VarInt(300),
        },
        Field {
            name: "custom_name",
            typ: Type::OptText(None),
        },
        Field {
            name: "custom_name_visible",
            typ: Type::Bool(false),
        },
        Field {
            name: "silent",
            typ: Type::Bool(false),
        },
        Field {
            name: "no_gravity",
            typ: Type::Bool(false),
        },
        Field {
            name: "pose",
            typ: Type::Pose,
        },
        Field {
            name: "frozen_ticks",
            typ: Type::VarInt(0),
        },
    ],
    events: &[],
};

const ABSTRACT_ARROW: Class = Class {
    name: "abstract_arrow",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "abstract_arrow_flags",
            typ: Type::BitFields(&[
                BitField {
                    name: "critical",
                    offset: 0,
                    default: false,
                },
                BitField {
                    name: "noclip",
                    offset: 1,
                    default: false,
                },
            ]),
        },
        Field {
            name: "piercing_level",
            typ: Type::Byte(0),
        },
    ],
    events: &[],
};

const ITEM_FRAME: Class = Class {
    name: "item_frame",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "item",
            typ: Type::Slot,
        },
        Field {
            name: "rotation",
            typ: Type::VarInt(0), // TODO: Direction enum?
        },
    ],
    events: &[],
};

const BOAT: Class = Class {
    name: "boat",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "last_hit_ticks",
            typ: Type::VarInt(0),
        },
        Field {
            name: "forward_direction",
            typ: Type::VarInt(1), // TODO: direction enum?
        },
        Field {
            name: "damage_taken",
            typ: Type::Float(0.0),
        },
        Field {
            name: "kind",
            typ: Type::BoatKind,
        },
        Field {
            name: "left_paddle_turning",
            typ: Type::Bool(false),
        },
        Field {
            name: "right_paddle_turning",
            typ: Type::Bool(false),
        },
        Field {
            name: "splash_timer",
            typ: Type::VarInt(0),
        },
    ],
    events: &[],
};

const LIVING_ENTITY: Class = Class {
    name: "living_entity",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "living_entity_flags",
            typ: Type::BitFields(&[
                BitField {
                    name: "hand_active",
                    offset: 0,
                    default: false,
                },
                BitField {
                    name: "active_hand",
                    offset: 1,
                    default: false,
                },
                BitField {
                    name: "riptide_spin_attack",
                    offset: 2,
                    default: false,
                },
            ]),
        },
        Field {
            name: "health",
            typ: Type::Float(1.0),
        },
        Field {
            name: "potion_effect_color",
            typ: Type::VarInt(0), // TODO: potion effect color type
        },
        Field {
            name: "potion_effect_ambient",
            typ: Type::Bool(false),
        },
        Field {
            name: "arrow_count",
            typ: Type::VarInt(0),
        },
        Field {
            name: "bee_stinger_count",
            typ: Type::VarInt(0),
        },
        Field {
            name: "bed_sleeping_position",
            typ: Type::OptBlockPos(None),
        },
    ],
    events: &[],
};

const MOB: Class = Class {
    name: "mob",
    inherit: Some(&LIVING_ENTITY),
    fields: &[Field {
        name: "mob_flags",
        typ: Type::BitFields(&[
            BitField {
                name: "ai_disabled",
                offset: 0,
                default: false,
            },
            BitField {
                name: "left_handed",
                offset: 1,
                default: false,
            },
            BitField {
                name: "aggressive",
                offset: 2,
                default: false,
            },
        ]),
    }],
    events: &[],
};

const AMBIENT_CREATURE: Class = Class {
    name: "ambient_creature",
    inherit: Some(&MOB),
    fields: &[],
    events: &[],
};

const PATHFINDER_MOB: Class = Class {
    name: "pathfinder_mob",
    inherit: Some(&MOB),
    fields: &[],
    events: &[],
};

const WATER_ANIMAL: Class = Class {
    name: "water_animal",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
    events: &[],
};

const ABSTRACT_FISH: Class = Class {
    name: "abstract_fish",
    inherit: Some(&WATER_ANIMAL),
    fields: &[Field {
        name: "from_bucket",
        typ: Type::Bool(false),
    }],
    events: &[],
};

const AGEABLE_MOB: Class = Class {
    name: "ageable_mob",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[Field {
        name: "is_baby",
        typ: Type::Bool(false),
    }],
    events: &[],
};

const ANIMAL: Class = Class {
    name: "animal",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
    events: &[],
};

const ABSTRACT_HORSE: Class = Class {
    name: "abstract_horse",
    inherit: Some(&ANIMAL),
    fields: &[
        Field {
            name: "horse_flags",
            typ: Type::BitFields(&[
                BitField {
                    name: "tame",
                    offset: 1, // Skip unused
                    default: false,
                },
                BitField {
                    name: "saddled",
                    offset: 2,
                    default: false,
                },
                BitField {
                    name: "bred",
                    offset: 3,
                    default: false,
                },
                BitField {
                    name: "eating",
                    offset: 4,
                    default: false,
                },
                BitField {
                    name: "rearing",
                    offset: 5,
                    default: false,
                },
                BitField {
                    name: "mouth_open",
                    offset: 6,
                    default: false,
                },
            ]),
        },
        Field {
            name: "owner",
            typ: Type::OptUuid,
        },
    ],
    events: &[],
};

const CHESTED_HORSE: Class = Class {
    name: "chested_horse",
    inherit: Some(&ABSTRACT_HORSE),
    fields: &[Field {
        name: "has_chest",
        typ: Type::Bool(false),
    }],
    events: &[],
};

const COW: Class = Class {
    name: "cow",
    inherit: Some(&ANIMAL),
    fields: &[],
    events: &[],
};

const TAMEABLE_ANIMAL: Class = Class {
    name: "tameable_animal",
    inherit: Some(&ANIMAL),
    fields: &[Field {
        name: "tameable_animal_flags",
        typ: Type::BitFields(&[
            BitField {
                name: "sitting",
                offset: 0,
                default: false,
            },
            BitField {
                name: "tamed",
                offset: 2, // Skip unused.
                default: false,
            },
        ]),
    }],
    events: &[],
};

const ABSTRACT_VILLAGER: Class = Class {
    name: "abstract_villager",
    inherit: Some(&AGEABLE_MOB),
    fields: &[Field {
        name: "head_shake_timer",
        typ: Type::VarInt(0),
    }],
    events: &[],
};

const ABSTRACT_GOLEM: Class = Class {
    name: "abstract_golem",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
    events: &[],
};

const MONSTER: Class = Class {
    name: "monster",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
    events: &[],
};

const BASE_PIGLIN: Class = Class {
    name: "base_piglin",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "zombification_immune",
        typ: Type::Bool(false),
    }],
    events: &[],
};

const GUARDIAN: Class = Class {
    name: "guardian",
    inherit: Some(&MONSTER),
    fields: &[
        Field {
            name: "retracting_spikes",
            typ: Type::Bool(false),
        },
        Field {
            name: "target",
            typ: Type::OptEntityId,
        },
    ],
    events: &[],
};

const RAIDER: Class = Class {
    name: "raider",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "celebrating",
        typ: Type::Bool(false),
    }],
    events: &[],
};

const ABSTRACT_ILLAGER: Class = Class {
    name: "abstract_illager",
    inherit: Some(&RAIDER),
    fields: &[],
    events: &[],
};

const SPELLCASTER_ILLAGER: Class = Class {
    name: "spellcaster_illager",
    inherit: Some(&ABSTRACT_ILLAGER),
    fields: &[Field {
        name: "spellcaster_state",
        typ: Type::Byte(0), /* TODO: Spell (0: none, 1: summon vex, 2: attack, 3: wololo, 4:
                             * disappear, 5: blindness) */
    }],
    events: &[],
};

const ABSTRACT_SKELETON: Class = Class {
    name: "abstract_skeleton",
    inherit: Some(&MONSTER),
    fields: &[],
    events: &[],
};

const SPIDER: Class = Class {
    name: "spider",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "spider_flags",
        typ: Type::BitFields(&[BitField {
            name: "climbing",
            offset: 0,
            default: false,
        }]),
    }],
    events: &[],
};

const ZOMBIE: Class = Class {
    name: "zombie",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "baby",
        typ: Type::Bool(false),
    }],
    events: &[],
};

const FLYING: Class = Class {
    name: "flying",
    inherit: Some(&MOB),
    fields: &[],
    events: &[],
};

const ABSTRACT_MINECART: Class = Class {
    name: "abstract minecart",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "shaking_power",
            typ: Type::VarInt(0),
        },
        Field {
            name: "shaking_direction",
            typ: Type::VarInt(1), // TODO: Refined type?
        },
        Field {
            name: "shaking_multiplier",
            typ: Type::Float(0.0),
        },
        Field {
            name: "custom_block_id",
            typ: Type::VarInt(0), // TODO: is this a BlockState?
        },
        Field {
            name: "custom_block_y_pos",
            typ: Type::VarInt(6), // TODO: measured in 16ths of a block. Refined type?
        },
        Field {
            name: "show_custom_block",
            typ: Type::Bool(false),
        },
    ],
    events: &[],
};

const ABSTRACT_MINECART_CONTAINER: Class = Class {
    name: "abstract_minecart_container",
    inherit: Some(&ABSTRACT_MINECART),
    fields: &[],
    events: &[],
};

const ENTITIES: &[Class] = &[
    Class {
        name: "allay",
        inherit: Some(&PATHFINDER_MOB),
        fields: &[], // TODO: fields?
        events: &[],
    },
    Class {
        // TODO: how is this defined?
        name: "leash_knot",
        inherit: None,
        fields: &[],
        events: &[],
    },
    Class {
        // TODO: how is this defined?
        name: "lightning_bolt",
        inherit: None,
        fields: &[],
        events: &[],
    },
    Class {
        name: "experience_orb",
        inherit: None,
        fields: &[],
        events: &[],
    },
    Class {
        name: "marker",
        inherit: None,
        fields: &[],
        events: &[],
    },
    Class {
        name: "item",
        inherit: Some(&BASE_ENTITY),
        fields: &[], // TODO: what are the fields?
        events: &[],
    },
    Class {
        name: "egg",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "ender_pearl",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "experience_bottle",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "potion",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "potion",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "snowball",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "eye_of_ender",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "falling_block",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "spawn_position",
            typ: Type::BlockPos(0, 0, 0),
        }],
        events: &[],
    },
    Class {
        name: "area_effect_cloud",
        inherit: Some(&BASE_ENTITY),
        fields: &[
            Field {
                name: "radius",
                typ: Type::Float(0.5),
            },
            Field {
                name: "color",
                typ: Type::VarInt(0),
            },
            Field {
                name: "ignore_radius",
                typ: Type::Bool(false),
            },
            Field {
                name: "particle",
                typ: Type::Particle,
            },
        ],
        events: &[],
    },
    Class {
        name: "fishing_bobber",
        inherit: Some(&BASE_ENTITY),
        fields: &[
            Field {
                name: "hooked_entity",
                typ: Type::OptEntityId,
            },
            Field {
                name: "catchable",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "arrow",
        inherit: Some(&ABSTRACT_ARROW),
        fields: &[Field {
            name: "color",
            typ: Type::VarInt(-1), // TODO: custom type
        }],
        events: &[],
    },
    Class {
        name: "spectral_arrow",
        inherit: Some(&ABSTRACT_ARROW),
        fields: &[],
        events: &[],
    },
    Class {
        name: "trident",
        inherit: Some(&ABSTRACT_ARROW),
        fields: &[
            Field {
                name: "loyalty_level",
                typ: Type::VarInt(0),
            },
            Field {
                name: "enchantment_glint",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    BOAT,
    Class {
        name: "chest_boat",
        inherit: Some(&BOAT),
        fields: &[],
        events: &[],
    },
    Class {
        name: "tadpole",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[],
        events: &[],
    },
    Class {
        name: "warden",
        inherit: Some(&MONSTER),
        fields: &[], // TODO: warden anger
        events: &[],
    },
    Class {
        name: "end_crystal",
        inherit: Some(&BASE_ENTITY),
        fields: &[
            Field {
                name: "beam_target",
                typ: Type::OptBlockPos(None),
            },
            Field {
                name: "show_bottom",
                typ: Type::Bool(true),
            },
        ],
        events: &[],
    },
    Class {
        name: "dragon_fireball",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
        events: &[],
    },
    Class {
        name: "small_fireball",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "fireball",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
        events: &[],
    },
    Class {
        name: "wither_skull",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "invulnerable",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "firework_rocket",
        inherit: Some(&BASE_ENTITY),
        fields: &[
            Field {
                name: "info",
                typ: Type::Slot,
            },
            Field {
                name: "used_by",
                typ: Type::OptEntityId,
            },
            Field {
                name: "shot_at_angle",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    ITEM_FRAME,
    Class {
        name: "glow_item_frame",
        inherit: Some(&ITEM_FRAME),
        fields: &[],
        events: &[],
    },
    Class {
        name: "painting",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "variant",
            typ: Type::PaintingKind,
        }],
        events: &[],
    },
    Class {
        name: "player",
        inherit: Some(&LIVING_ENTITY),
        fields: &[
            Field {
                name: "additional_hearts",
                typ: Type::Float(0.0),
            },
            Field {
                name: "score",
                typ: Type::VarInt(0),
            },
            Field {
                name: "displayed_skin_parts",
                typ: Type::BitFields(&[
                    BitField {
                        name: "cape_enabled",
                        offset: 0,
                        default: false,
                    },
                    BitField {
                        name: "jacket_enabled",
                        offset: 1,
                        default: false,
                    },
                    BitField {
                        name: "left_sleeve_enabled",
                        offset: 2,
                        default: false,
                    },
                    BitField {
                        name: "right_sleeve_enabled",
                        offset: 3,
                        default: false,
                    },
                    BitField {
                        name: "left_pants_leg_enabled",
                        offset: 4,
                        default: false,
                    },
                    BitField {
                        name: "right_pants_leg_enabled",
                        offset: 5,
                        default: false,
                    },
                    BitField {
                        name: "hat_enabled",
                        offset: 6,
                        default: false,
                    },
                ]),
            },
            Field {
                name: "main_hand",
                typ: Type::MainHand,
            },
            Field {
                name: "left_shoulder_entity_data",
                typ: Type::Nbt,
            },
            Field {
                name: "right_shoulder_entity_data",
                typ: Type::Nbt,
            },
            Field {
                name: "global_position",
                typ: Type::OptGlobalPosition,
            },
        ],
        events: &[
            Event::SwingMainArm,
            Event::TakeDamage,
            Event::LeaveBed,
            Event::SwingOffhand,
            Event::CriticalEffect,
            Event::MagicCriticalEffect,
        ],
    },
    Class {
        name: "armor_stand",
        inherit: Some(&LIVING_ENTITY),
        fields: &[
            Field {
                name: "armor_stand_flags",
                typ: Type::BitFields(&[
                    BitField {
                        name: "small",
                        offset: 0,
                        default: false,
                    },
                    BitField {
                        name: "arms",
                        offset: 1,
                        default: false,
                    },
                    BitField {
                        name: "no_baseplate",
                        offset: 2,
                        default: false,
                    },
                    BitField {
                        name: "marker",
                        offset: 3,
                        default: false,
                    },
                ]),
            },
            Field {
                name: "head_rotation",
                typ: Type::ArmorStandRotations(0.0, 0.0, 0.0),
            },
            Field {
                name: "body_rotation",
                typ: Type::ArmorStandRotations(0.0, 0.0, 0.0),
            },
            Field {
                name: "left_arm_rotation",
                typ: Type::ArmorStandRotations(-10.0, 0.0, -10.0),
            },
            Field {
                name: "right_arm_rotation",
                typ: Type::ArmorStandRotations(-15.0, 0.0, -10.0),
            },
            Field {
                name: "left_leg_rotation",
                typ: Type::ArmorStandRotations(-1.0, 0.0, -1.0),
            },
            Field {
                name: "right_leg_rotation",
                typ: Type::ArmorStandRotations(1.0, 0.0, 1.0),
            },
        ],
        events: &[],
    },
    Class {
        name: "bat",
        inherit: Some(&AMBIENT_CREATURE),
        fields: &[Field {
            name: "bat_flags",
            typ: Type::BitFields(&[BitField {
                name: "hanging",
                offset: 0,
                default: false,
            }]),
        }],
        events: &[],
    },
    Class {
        name: "squid",
        inherit: Some(&WATER_ANIMAL),
        fields: &[],
        events: &[],
    },
    Class {
        // TODO: How is glow squid defined? This is a guess.
        name: "glow_squid",
        inherit: Some(&WATER_ANIMAL),
        fields: &[],
        events: &[],
    },
    Class {
        name: "dolphin",
        inherit: Some(&WATER_ANIMAL),
        fields: &[
            Field {
                name: "treasure_position",
                typ: Type::BlockPos(0, 0, 0),
            },
            Field {
                name: "has_fish",
                typ: Type::Bool(false),
            },
            Field {
                name: "moisture_level",
                typ: Type::VarInt(2400),
            },
        ],
        events: &[],
    },
    Class {
        name: "cod",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[],
        events: &[],
    },
    Class {
        name: "pufferfish",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[Field {
            name: "puff_state",
            typ: Type::VarInt(0), // TODO: PuffState in the range [0, 2]. (Bounded int?)
        }],
        events: &[],
    },
    Class {
        name: "salmon",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[],
        events: &[],
    },
    Class {
        name: "tropical_fish",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: TropicalFishVariant enum
        }],
        events: &[],
    },
    Class {
        name: "horse",
        inherit: Some(&ABSTRACT_HORSE),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: HorseVariant enum
        }],
        events: &[],
    },
    Class {
        name: "zombie_horse",
        inherit: Some(&ABSTRACT_HORSE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "skeleton_horse",
        inherit: Some(&ABSTRACT_HORSE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "donkey",
        inherit: Some(&CHESTED_HORSE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "llama",
        inherit: Some(&CHESTED_HORSE),
        fields: &[
            Field {
                name: "strength",
                typ: Type::VarInt(0), // TODO: upper bound?
            },
            Field {
                name: "carpet_color",
                typ: Type::VarInt(-1), // TODO: Carpet color enum.
            },
            Field {
                name: "variant",
                typ: Type::VarInt(0), // TODO: Llama variant enum.
            },
        ],
        events: &[],
    },
    Class {
        name: "trader_llama",
        inherit: None, // TODO: really?
        fields: &[],
        events: &[],
    },
    Class {
        name: "mule",
        inherit: Some(&CHESTED_HORSE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "axolotl",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "variant",
                typ: Type::VarInt(0), // TODO: AxolotlVariant enum.
            },
            Field {
                name: "playing_dead",
                typ: Type::Bool(false),
            },
            Field {
                name: "from_bucket",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "bee",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "bee_flags",
                typ: Type::BitFields(&[
                    BitField {
                        name: "angry",
                        offset: 1, // Skip unused.
                        default: false,
                    },
                    BitField {
                        name: "stung",
                        offset: 2,
                        default: false,
                    },
                    BitField {
                        name: "nectar",
                        offset: 3,
                        default: false,
                    },
                ]),
            },
            Field {
                name: "anger_ticks",
                typ: Type::VarInt(0),
            },
        ],
        events: &[],
    },
    Class {
        name: "fox",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "variant",
                typ: Type::VarInt(0), // TODO: 0 for red, 1 for snow
            },
            Field {
                name: "fox_flags",
                typ: Type::BitFields(&[
                    BitField {
                        name: "sitting",
                        offset: 0,
                        default: false,
                    },
                    BitField {
                        name: "fox_crouching",
                        offset: 2, // Skip unused
                        default: false,
                    },
                    BitField {
                        name: "interested",
                        offset: 3,
                        default: false,
                    },
                    BitField {
                        name: "pouncing",
                        offset: 4,
                        default: false,
                    },
                    BitField {
                        name: "sleeping",
                        offset: 5,
                        default: false,
                    },
                    BitField {
                        name: "faceplanted",
                        offset: 6,
                        default: false,
                    },
                    BitField {
                        name: "defending",
                        offset: 7,
                        default: false,
                    },
                ]),
            },
            // TODO: what are these UUIDs?
            Field {
                name: "first_uuid",
                typ: Type::OptUuid,
            },
            Field {
                name: "second_uuid",
                typ: Type::OptUuid,
            },
        ],
        events: &[],
    },
    Class {
        name: "frog",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "variant",
                typ: Type::FrogKind,
            },
            Field {
                name: "tongue_target",
                typ: Type::VarInt(0),
            },
        ],
        events: &[],
    },
    Class {
        name: "ocelot",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "trusting",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "panda",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "breed_timer",
                typ: Type::VarInt(0),
            },
            Field {
                name: "sneeze_timer",
                typ: Type::VarInt(0),
            },
            Field {
                name: "eat_timer",
                typ: Type::VarInt(0),
            },
            Field {
                name: "main_gene",
                typ: Type::Byte(0),
            },
            Field {
                name: "hidden_gene",
                typ: Type::Byte(0),
            },
            Field {
                name: "panda_flags",
                typ: Type::BitFields(&[
                    BitField {
                        name: "sneezing",
                        offset: 1, // Skip unused.
                        default: false,
                    },
                    BitField {
                        name: "rolling",
                        offset: 2,
                        default: false,
                    },
                    BitField {
                        name: "sitting",
                        offset: 3,
                        default: false,
                    },
                    BitField {
                        name: "on_back",
                        offset: 4,
                        default: false,
                    },
                ]),
            },
        ],
        events: &[],
    },
    Class {
        name: "pig",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "has_saddle",
                typ: Type::Bool(false),
            },
            Field {
                name: "boost_timer",
                typ: Type::VarInt(0),
            },
        ],
        events: &[],
    },
    Class {
        name: "rabbit",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: rabbit variant enum.
        }],
        events: &[],
    },
    Class {
        name: "turtle",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "home_position",
                typ: Type::BlockPos(0, 0, 0),
            },
            Field {
                name: "has_egg",
                typ: Type::Bool(false),
            },
            Field {
                name: "laying_egg",
                typ: Type::Bool(false),
            },
            Field {
                name: "travel_pos",
                typ: Type::BlockPos(0, 0, 0),
            },
            Field {
                name: "going_home",
                typ: Type::Bool(false),
            },
            Field {
                name: "travelling",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "polar_bear",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "standing_up",
            typ: Type::Bool(true),
        }],
        events: &[],
    },
    Class {
        name: "chicken",
        inherit: Some(&ANIMAL),
        fields: &[],
        events: &[],
    },
    COW,
    Class {
        name: "hoglin",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "zombification_immune",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "mooshroom",
        inherit: Some(&COW),
        fields: &[Field {
            name: "variant",
            typ: Type::String("red"), // TODO: "red" or "brown" enum.
        }],
        events: &[],
    },
    Class {
        name: "sheep",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "sheep_state",
            typ: Type::Byte(0), // TODO: sheep state type.
        }],
        events: &[],
    },
    Class {
        name: "goat",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "screaming",
                typ: Type::Bool(false),
            },
            Field {
                name: "left_horn",
                typ: Type::Bool(true),
            },
            Field {
                name: "right_horn",
                typ: Type::Bool(true),
            },
        ],
        events: &[],
    },
    Class {
        name: "strider",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "boost_timer",
                typ: Type::VarInt(0),
            },
            Field {
                name: "shaking",
                typ: Type::Bool(false),
            },
            Field {
                name: "saddle",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "cat",
        inherit: Some(&TAMEABLE_ANIMAL),
        fields: &[
            Field {
                name: "variant",
                typ: Type::CatKind,
            },
            Field {
                name: "lying",
                typ: Type::Bool(false),
            },
            Field {
                name: "relaxed",
                typ: Type::Bool(false),
            },
            Field {
                name: "collar_color",
                typ: Type::VarInt(14), // TODO: dye color enum.
            },
        ],
        events: &[],
    },
    Class {
        name: "wolf",
        inherit: Some(&TAMEABLE_ANIMAL),
        fields: &[
            Field {
                name: "begging",
                typ: Type::Bool(false),
            },
            Field {
                name: "collar_color", // TODO: dye color enum
                typ: Type::VarInt(14),
            },
            Field {
                name: "anger_timer",
                typ: Type::VarInt(0),
            },
        ],
        events: &[],
    },
    Class {
        name: "parrot",
        inherit: Some(&TAMEABLE_ANIMAL),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: parrot variant enum.
        }],
        events: &[],
    },
    Class {
        name: "villager",
        inherit: Some(&ABSTRACT_VILLAGER),
        fields: &[Field {
            name: "villager_data",
            typ: Type::VillagerData,
        }],
        events: &[],
    },
    Class {
        name: "wandering_trader",
        inherit: Some(&ABSTRACT_VILLAGER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "iron_golem",
        inherit: Some(&ABSTRACT_GOLEM),
        fields: &[Field {
            name: "iron_golem_flags",
            typ: Type::BitFields(&[BitField {
                name: "player_created",
                offset: 0,
                default: false,
            }]),
        }],
        events: &[],
    },
    Class {
        name: "snow_golem",
        inherit: Some(&ABSTRACT_GOLEM),
        fields: &[Field {
            name: "snow_golem_flags",
            typ: Type::BitFields(&[BitField {
                name: "pumpkin_hat",
                offset: 4,
                default: true,
            }]),
        }],
        events: &[],
    },
    Class {
        name: "shulker",
        inherit: Some(&ABSTRACT_GOLEM),
        fields: &[
            Field {
                name: "attach_face",
                typ: Type::Direction,
            },
            Field {
                name: "attachment_position",
                typ: Type::OptBlockPos(None),
            },
            Field {
                name: "shield_height",
                typ: Type::Byte(0),
            },
            Field {
                name: "color",
                typ: Type::Byte(10), // TODO: dye color enum
            },
        ],
        events: &[],
    },
    Class {
        // TODO: how is this defined?
        name: "shulker_bullet",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
        events: &[],
    },
    Class {
        name: "piglin",
        inherit: Some(&BASE_PIGLIN),
        fields: &[
            Field {
                name: "baby",
                typ: Type::Bool(false),
            },
            Field {
                name: "charging_crossbow",
                typ: Type::Bool(false),
            },
            Field {
                name: "dancing",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "piglin_brute",
        inherit: Some(&BASE_PIGLIN),
        fields: &[],
        events: &[],
    },
    Class {
        name: "blaze",
        inherit: Some(&MONSTER),
        fields: &[Field {
            name: "blaze_flags",
            typ: Type::BitFields(&[BitField {
                name: "blaze_on_fire", // TODO: better name for this?
                offset: 0,
                default: false,
            }]),
        }],
        events: &[],
    },
    Class {
        name: "creeper",
        inherit: Some(&MONSTER),
        fields: &[
            Field {
                name: "creeper_state",
                typ: Type::VarInt(-1), // TODO -1 for idle, +1 for fuse.
            },
            Field {
                name: "charged",
                typ: Type::Bool(false),
            },
            Field {
                name: "ignited",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "endermite",
        inherit: Some(&MONSTER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "giant",
        inherit: Some(&MONSTER),
        fields: &[],
        events: &[],
    },
    GUARDIAN,
    Class {
        name: "elder_guardian",
        inherit: Some(&GUARDIAN),
        fields: &[],
        events: &[],
    },
    Class {
        name: "silverfish",
        inherit: Some(&MONSTER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "vindicator",
        inherit: Some(&ABSTRACT_ILLAGER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "pillager",
        inherit: Some(&ABSTRACT_ILLAGER),
        fields: &[Field {
            name: "charging",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "evoker",
        inherit: Some(&SPELLCASTER_ILLAGER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "illusioner",
        inherit: Some(&SPELLCASTER_ILLAGER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "ravager",
        inherit: Some(&RAIDER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "evoker_fangs",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
        events: &[],
    },
    Class {
        name: "witch",
        inherit: Some(&RAIDER),
        fields: &[Field {
            name: "drinking_potion",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "vex",
        inherit: Some(&MONSTER),
        fields: &[Field {
            name: "vex_flags",
            typ: Type::BitFields(&[BitField {
                name: "attacking",
                offset: 0,
                default: false,
            }]),
        }],
        events: &[],
    },
    Class {
        name: "skeleton",
        inherit: Some(&ABSTRACT_SKELETON),
        fields: &[],
        events: &[],
    },
    Class {
        name: "wither_skeleton",
        inherit: Some(&ABSTRACT_SKELETON),
        fields: &[],
        events: &[],
    },
    Class {
        name: "stray",
        inherit: Some(&ABSTRACT_SKELETON),
        fields: &[],
        events: &[],
    },
    SPIDER,
    Class {
        name: "cave_spider",
        inherit: Some(&SPIDER), // TODO: does cave_spider inherit from spider?
        fields: &[],
        events: &[],
    },
    Class {
        name: "wither",
        inherit: Some(&MONSTER),
        fields: &[
            // TODO: are these actually OptEntityId, or something else?
            Field {
                name: "center_head_target",
                typ: Type::OptEntityId,
            },
            Field {
                name: "left_head_target",
                typ: Type::OptEntityId,
            },
            Field {
                name: "right_head_target",
                typ: Type::OptEntityId,
            },
            Field {
                name: "invulnerable_time",
                typ: Type::VarInt(0),
            },
        ],
        events: &[],
    },
    Class {
        name: "zoglin",
        inherit: Some(&MONSTER),
        fields: &[Field {
            name: "baby",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    ZOMBIE,
    Class {
        name: "zombie_villager",
        inherit: Some(&ZOMBIE),
        fields: &[
            Field {
                name: "converting",
                typ: Type::Bool(false),
            },
            Field {
                name: "villager_data",
                typ: Type::VillagerData,
            },
        ],
        events: &[],
    },
    Class {
        name: "husk",
        inherit: Some(&ZOMBIE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "drowned",
        inherit: Some(&ZOMBIE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "zombified_piglin",
        inherit: Some(&ZOMBIE),
        fields: &[],
        events: &[],
    },
    Class {
        name: "enderman",
        inherit: Some(&MONSTER),
        fields: &[
            Field {
                name: "carried_block",
                typ: Type::BlockState,
            },
            Field {
                name: "screaming",
                typ: Type::Bool(false),
            },
            Field {
                name: "staring",
                typ: Type::Bool(false),
            },
        ],
        events: &[],
    },
    Class {
        name: "ender_dragon",
        inherit: Some(&MOB),
        fields: &[Field {
            name: "phase",
            typ: Type::VarInt(10), // TODO: dragon phase enum
        }],
        events: &[],
    },
    Class {
        name: "ghast",
        inherit: Some(&FLYING),
        fields: &[Field {
            name: "attacking",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "phantom",
        inherit: Some(&FLYING),
        fields: &[Field {
            name: "size",
            typ: Type::VarInt(0),
        }],
        events: &[],
    },
    Class {
        name: "slime",
        inherit: Some(&MOB),
        fields: &[Field {
            name: "size",
            typ: Type::VarInt(1), // TODO: bounds?
        }],
        events: &[],
    },
    Class {
        name: "magma_cube",
        inherit: Some(&MOB),
        fields: &[Field {
            name: "size",
            typ: Type::VarInt(1),
        }],
        events: &[],
    },
    Class {
        name: "llama_spit",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
        events: &[],
    },
    Class {
        name: "minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
        events: &[],
    },
    Class {
        name: "hopper_minecart",
        inherit: Some(&ABSTRACT_MINECART_CONTAINER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "chest_minecart",
        inherit: Some(&ABSTRACT_MINECART_CONTAINER),
        fields: &[],
        events: &[],
    },
    Class {
        name: "furnace_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[Field {
            name: "has_fuel",
            typ: Type::Bool(false),
        }],
        events: &[],
    },
    Class {
        name: "tnt_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
        events: &[],
    },
    Class {
        name: "spawner_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
        events: &[],
    },
    Class {
        name: "command_block_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[
            Field {
                name: "command",
                typ: Type::String(""),
            },
            Field {
                name: "last_output",
                typ: Type::Text,
            },
        ],
        events: &[],
    },
    Class {
        name: "tnt",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "fuse_timer",
            typ: Type::VarInt(80),
        }],
        events: &[],
    },
];

pub fn build() -> anyhow::Result<()> {
    // Sort the entities in ID order, where the IDs are obtained from entities.json.
    let entities = {
        let entities: HashMap<_, _> = ENTITIES.iter().map(|c| (c.name, c)).collect();

        #[derive(Deserialize)]
        struct JsonEntity {
            id: usize,
            name: String,
        }

        let json_entities: Vec<JsonEntity> =
            serde_json::from_str(include_str!("../data/entities.json"))?;

        let mut res = Vec::new();

        for (i, e) in json_entities.iter().enumerate() {
            assert_eq!(e.id, i);

            let name = e.name.as_str();

            res.push(
                *entities
                    .get(name)
                    .with_context(|| format!("entity \"{name}\" was not defined"))?,
            );
        }

        assert_eq!(json_entities.len(), entities.len());

        res
    };

    let mut all_classes = BTreeMap::new();
    for mut class in entities.iter().cloned() {
        while let None = all_classes.insert(class.name, class) {
            match class.inherit {
                Some(parent) => class = parent,
                None => break,
            }
        }
    }

    let entity_kind_variants = entities
        .iter()
        .map(|c| ident(c.name.to_pascal_case()))
        .collect::<Vec<_>>();

    let entity_structs = entities.iter().map(|&class| {
       let mut fields = Vec::new();
       class.collect_fields(&mut fields);

       let name = ident(class.name.to_pascal_case());
       let struct_fields = fields.iter().map(|&f| {
           let name = ident(f.name.to_snake_case());
           let typ = match f.typ {
               Type::BitFields(_) => quote! { u8 },
               Type::Byte(_) => quote! { u8 },
               Type::VarInt(_) => quote! { VarInt },
               Type::Float(_) => quote! { f32 },
               Type::String(_) => quote! { Box<str> },
               Type::Text => quote! { Box<Text> },
               Type::OptText(_) => quote! { Option<Box<Text>> },
               Type::Slot => quote! { () }, // TODO
               Type::Bool(_) => quote! { bool },
               Type::ArmorStandRotations(_, _, _) => quote! { ArmorStandRotations },
               Type::BlockPos(_, _, _) => quote! { BlockPos },
               Type::OptBlockPos(_) => quote! { Option<BlockPos> },
               Type::Direction => quote! { Direction },
               Type::OptUuid => quote! { Option<Uuid> },
               Type::BlockState => quote! { BlockState },
               Type::Nbt => quote! { nbt::Blob },
               Type::Particle => quote! { () }, // TODO
               Type::VillagerData => quote! { VillagerData },
               Type::OptEntityId => quote! { Option<EntityId> },
               Type::Pose => quote! { Pose },
               Type::CatKind => quote! { CatKind },
               Type::FrogKind => quote! { FrogKind },
               Type::OptGlobalPosition => quote! { () }, // TODO
               Type::PaintingKind => quote! { PaintingKind },
               Type::BoatKind => quote! { BoatKind },
               Type::MainHand => quote! { MainHand },
           };
           quote! {
               #name: #typ,
           }
       });

       let constructor_fields = fields.iter().map(|field| {
           let name = ident(field.name.to_snake_case());
           let val = field.typ.default_expr();
           quote! {
               #name: #val,
           }
       });
        
        let getter_setters = 
            fields
            .iter()
            .enumerate()
            .map(|(field_offset, field)| {
                let name = ident(field.name.to_snake_case());
                let getter_name = ident(format!("get_{}", name.to_string()));
                let setter_name = ident(format!("set_{}", name.to_string()));

                let field_offset = field_offset as u32;

                // TODO: documentation on methods.

                let standard_getter_setter = |type_name: TokenStream| quote! {
                    pub fn #getter_name(&self) -> #type_name {
                        self.#name
                    }

                    pub fn #setter_name(&mut self, #name: #type_name) {
                        if self.#name != #name {
                            self.modified_flags |= 1 << #field_offset;
                        }

                        self.#name = #name;
                    }
                };

                match field.typ {
                    Type::BitFields(bfs) => bfs
                        .iter()
                        .map(|bf| {
                            let bit_name = ident(bf.name.to_snake_case());

                            let getter_name = ident(format!("get_{}", bit_name.to_string()));
                            let setter_name = ident(format!("set_{}", bit_name.to_string()));

                            let offset = bf.offset;

                            quote! {
                                pub fn #getter_name(&self) -> bool {
                                    (self.#name >> #offset) & 1 == 1
                                }

                                pub fn #setter_name(&mut self, #bit_name: bool) {
                                    if self.#getter_name() != #bit_name {
                                        self.#name = (self.#name & !(1 << #offset)) | ((#bit_name as u8) << #offset);
                                        self.modified_flags |= 1 << #field_offset;
                                    }
                                }
                            }
                        })
                        .collect(),
                    Type::Byte(_) => standard_getter_setter(quote!(u8)),
                    Type::VarInt(_) => quote! {
                        pub fn #getter_name(&self) -> i32 {
                            self.#name.0
                        }

                        pub fn #setter_name(&mut self, #name: i32) {
                            if self.#name.0 != #name {
                                self.modified_flags |= 1 << #field_offset;
                            }

                            self.#name = VarInt(#name);
                        }
                    },
                    Type::Float(_) => standard_getter_setter(quote!(f32)),
                    Type::String(_) => quote! {
                        pub fn #getter_name(&self) -> &str {
                            &self.#name
                        }

                        pub fn #setter_name(&mut self, #name: impl Into<Box<str>>) {
                            let #name = #name.into();

                            if self.#name != #name {
                                self.modified_flags |= 1 << #field_offset;
                            }

                            self.#name = #name;
                        }
                    },
                    Type::Text => quote! {
                        pub fn #getter_name(&self) -> &Text {
                            &self.#name
                        }

                        pub fn #setter_name(&mut self, #name: impl Into<Text>) {
                            let #name = Box::new(#name.into());

                            if self.#name != #name {
                                self.modified_flags |= 1 << #field_offset;
                            }

                            self.#name = #name;
                        }
                    },
                    Type::OptText(_) => quote! {
                        pub fn #getter_name(&self) -> Option<&Text> {
                            self.#name.as_deref()
                        }

                        pub fn #setter_name(&mut self, #name: Option<impl Into<Text>>) {
                            let #name = #name.map(|x| Box::new(x.into()));

                            if self.#name != #name {
                                self.modified_flags |= 1 << #field_offset;
                            }

                            self.#name = #name;
                        }
                    },
                    Type::Slot => quote! {}, // TODO
                    Type::Bool(_) => standard_getter_setter(quote!(bool)),
                    Type::ArmorStandRotations(_, _, _) => standard_getter_setter(quote!(ArmorStandRotations)),
                    Type::BlockPos(_, _, _) => standard_getter_setter(quote!(BlockPos)),
                    Type::OptBlockPos(_) => standard_getter_setter(quote!(Option<BlockPos>)),
                    Type::Direction => standard_getter_setter(quote!(Direction)),
                    Type::OptUuid => standard_getter_setter(quote!(Option<Uuid>)),
                    Type::BlockState => standard_getter_setter(quote!(BlockState)),
                    Type::Nbt => quote! {
                        pub fn #getter_name(&self) -> &nbt::Blob {
                            &self.#name
                        }

                        pub fn #setter_name(&mut self, #name: nbt::Blob) {
                            if self.#name != #name {
                                self.modified_flags |= 1 << #field_offset;
                            }

                            self.#name = #name;
                        }
                    },
                    Type::Particle => quote! {}, // TODO
                    Type::VillagerData => standard_getter_setter(quote!(VillagerData)),
                    Type::OptEntityId => standard_getter_setter(quote!(Option<EntityId>)),
                    Type::Pose => standard_getter_setter(quote!(Pose)),
                    Type::CatKind => standard_getter_setter(quote!(CatKind)),
                    Type::FrogKind => standard_getter_setter(quote!(FrogKind)),
                    Type::OptGlobalPosition => quote! {}, // TODO
                    Type::PaintingKind => standard_getter_setter(quote!(PaintingKind)),
                    Type::BoatKind => standard_getter_setter(quote!(BoatKind)),
                    Type::MainHand => standard_getter_setter(quote!(MainHand)),
                }
            })
            .collect::<TokenStream>();

        let mut events = Vec::new();
        class.collect_events(&mut events);
        
        let trigger_methods = events.into_iter().map(|event| {
            let name = ident("trigger_".to_owned() + &event.snake_case_name());
            let code = event as u8;
            quote! {
                pub fn #name(&mut self) {
                    self.events.push(#code);
                }
            }
        }).collect::<TokenStream>();

        let initial_metadata_fields = fields.iter().enumerate().map(|(idx, f)| {
            let name = ident(f.name.to_snake_case());
            let default = f.typ.default_expr();
            let index: u8 = idx.try_into().unwrap();
            let type_id = f.typ.type_id();
            quote! {
                if self.#name != #default {
                    data.push(#index);
                    VarInt(#type_id).encode(data).unwrap();
                    self.#name.encode(data).unwrap();
                }
            }
        }).collect::<TokenStream>();

        let updated_metadata_fields = fields.iter().enumerate().map(|(idx, f)| {
            let name = ident(f.name.to_snake_case());
            let u8_index: u8 = idx.try_into().unwrap();
            let u32_index = idx as u32;
            let type_id = f.typ.type_id();
            quote! {
                if (self.modified_flags >> #u32_index) & 1 == 1 {
                    data.push(#u8_index);
                    VarInt(#type_id).encode(data).unwrap();
                    self.#name.encode(data).unwrap();
                }
            }
        }).collect::<TokenStream>();

        quote! {
            pub struct #name {
                events: Vec<u8>,
                /// Contains a set bit for each modified metadata field.
                modified_flags: u32,
                #(#struct_fields)*
            }

            impl #name {
                pub(crate) fn new() -> Self {
                    Self {
                        events: Vec::new(),
                        modified_flags: 0,
                        #(#constructor_fields)*
                    }
                }

                #getter_setters

                #trigger_methods

                pub(crate) fn initial_metadata(&self, #[allow(unused)] data: &mut Vec<u8>) {
                    #initial_metadata_fields
                }

                pub(crate) fn updated_metadata(&self, #[allow(unused)] data: &mut Vec<u8>) {
                    if self.modified_flags == 0 {
                        return;
                    }

                    #updated_metadata_fields
                }

                pub(crate) fn event_codes(&self) -> &[u8] {
                    &self.events
                }

                pub(crate) fn clear_modifications(&mut self) {
                    self.events.clear();
                    self.modified_flags = 0;
                }
            }
        }
    });

    let finished = quote! {
        /// Identifies a type of entity, such as `chicken`, `zombie` or `item`.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum EntityKind {
            #(#entity_kind_variants,)*
        }

        impl Default for EntityKind {
            fn default() -> Self {
                Self::Marker
            }
        }

        #(#entity_structs)*

        /// An enum encoding the type of an entity along with any data specific to that entity type.
        pub enum EntityData {
            #(#entity_kind_variants(#entity_kind_variants),)*
        }

        impl EntityData {
            pub(super) fn new(kind: EntityKind) -> Self {
                match kind {
                    #(EntityKind::#entity_kind_variants => Self::#entity_kind_variants(#entity_kind_variants::new()),)*
                }
            }

            pub(super) fn kind(&self) -> EntityKind {
                match self {
                    #(Self::#entity_kind_variants(_) => EntityKind::#entity_kind_variants,)*
                }
            }

            pub(super) fn initial_metadata(&self) -> Option<Vec<u8>> {
                let mut data = Vec::new();

                match self {
                    #(Self::#entity_kind_variants(e) => e.initial_metadata(&mut data),)*
                }

                if data.is_empty() {
                    None
                } else {
                    data.push(0xff);
                    Some(data)
                }
            }

            pub(super) fn updated_metadata(&self) -> Option<Vec<u8>> {
                let mut data = Vec::new();

                match self {
                    #(Self::#entity_kind_variants(e) => e.updated_metadata(&mut data),)*
                }

                if data.is_empty() {
                    None
                } else {
                    data.push(0xff);
                    Some(data)
                }
            }

            pub(crate) fn event_codes(&self) -> &[u8] {
                match self {
                    #(Self::#entity_kind_variants(e) => e.event_codes(),)*
                }
            }

            pub(super) fn clear_modifications(&mut self) {
                match self {
                    #(Self::#entity_kind_variants(e) => e.clear_modifications(),)*
                }
            }
        }
    };

    write_to_out_path("entity.rs", &finished.to_string())
}
