//! See: <https://wiki.vg/Entity_metadata>

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
}

struct Field {
    name: &'static str,
    typ: Type,
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
    OptVarInt,
    Pose,
    // ==== Specialized ==== //
    OptEntityId,
    BoatVariant,
    MainHand,
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
            name: "base_entity_bits",
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
};

const ABSTRACT_ARROW: Class = Class {
    name: "abstract_arrow",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "abstract_arrow_bits",
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
};

const LIVING_ENTITY: Class = Class {
    name: "living_entity",
    inherit: Some(&BASE_ENTITY),
    fields: &[
        Field {
            name: "living_entity_bits",
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
};

const MOB: Class = Class {
    name: "mob",
    inherit: Some(&LIVING_ENTITY),
    fields: &[Field {
        name: "mob_bits",
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
};

const AMBIENT_CREATURE: Class = Class {
    name: "ambient_creature",
    inherit: Some(&MOB),
    fields: &[],
};

const PATHFINDER_MOB: Class = Class {
    name: "pathfinder_mob",
    inherit: Some(&MOB),
    fields: &[],
};

const WATER_ANIMAL: Class = Class {
    name: "water_animal",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
};

const ABSTRACT_FISH: Class = Class {
    name: "abstract_fish",
    inherit: Some(&WATER_ANIMAL),
    fields: &[Field {
        name: "from_bucket",
        typ: Type::Bool(false),
    }],
};

const AGEABLE_MOB: Class = Class {
    name: "ageable_mob",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[Field {
        name: "is_baby",
        typ: Type::Bool(false),
    }],
};

const ANIMAL: Class = Class {
    name: "animal",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
};

const ABSTRACT_HORSE: Class = Class {
    name: "abstract_horse",
    inherit: Some(&ANIMAL),
    fields: &[
        Field {
            name: "horse_bits",
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
};

const CHESTED_HORSE: Class = Class {
    name: "chested_horse",
    inherit: Some(&ABSTRACT_HORSE),
    fields: &[Field {
        name: "has_chest",
        typ: Type::Bool(false),
    }],
};

const COW: Class = Class {
    name: "cow",
    inherit: Some(&ANIMAL),
    fields: &[],
};

const TAMEABLE_ANIMAL: Class = Class {
    name: "tameable_animal",
    inherit: Some(&ANIMAL),
    fields: &[Field {
        name: "tameable_animal_bits",
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
};

const ABSTRACT_VILLAGER: Class = Class {
    name: "abstract_villager",
    inherit: Some(&AGEABLE_MOB),
    fields: &[Field {
        name: "head_shake_timer",
        typ: Type::VarInt(0),
    }],
};

const ABSTRACT_GOLEM: Class = Class {
    name: "abstract_golem",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
};

const MONSTER: Class = Class {
    name: "monster",
    inherit: Some(&PATHFINDER_MOB),
    fields: &[],
};

const BASE_PIGLIN: Class = Class {
    name: "base_piglin",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "zombification_immune",
        typ: Type::Bool(false),
    }],
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
};

const RAIDER: Class = Class {
    name: "raider",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "celebrating",
        typ: Type::Bool(false),
    }],
};

const ABSTRACT_ILLAGER: Class = Class {
    name: "abstract_illager",
    inherit: Some(&RAIDER),
    fields: &[],
};

const SPELLCASTER_ILLAGER: Class = Class {
    name: "spellcaster_illager",
    inherit: Some(&ABSTRACT_ILLAGER),
    fields: &[Field {
        name: "spellcaster_state",
        typ: Type::Byte(0), /* TODO: Spell (0: none, 1: summon vex, 2: attack, 3: wololo, 4:
                             * disappear, 5: blindness) */
    }],
};

const ABSTRACT_SKELETON: Class = Class {
    name: "abstract_skeleton",
    inherit: Some(&MONSTER),
    fields: &[],
};

const SPIDER: Class = Class {
    name: "spider",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "spider_bits",
        typ: Type::BitFields(&[BitField {
            name: "climbing",
            offset: 0,
            default: false,
        }]),
    }],
};

const ZOMBIE: Class = Class {
    name: "zombie",
    inherit: Some(&MONSTER),
    fields: &[Field {
        name: "baby",
        typ: Type::Bool(false),
    }],
};

const FLYING: Class = Class {
    name: "flying",
    inherit: Some(&MOB),
    fields: &[],
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
};

const ABSTRACT_MINECART_CONTAINER: Class = Class {
    name: "abstract_minecart_container",
    inherit: Some(&ABSTRACT_MINECART),
    fields: &[],
};

const ENTITIES: &[Class] = &[
    Class {
        // TODO: how is this defined?
        name: "leash_knot",
        inherit: None,
        fields: &[],
    },
    Class {
        // TODO: how is this defined?
        name: "lightning_bolt",
        inherit: None,
        fields: &[],
    },
    Class {
        name: "experience_orb",
        inherit: None,
        fields: &[],
    },
    Class {
        name: "painting",
        inherit: None,
        fields: &[],
    },
    Class {
        name: "marker",
        inherit: None,
        fields: &[],
    },
    Class {
        name: "item",
        inherit: Some(&BASE_ENTITY),
        fields: &[], // TODO: what are the fields?
    },
    Class {
        name: "egg",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "ender_pearl",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "experience_bottle",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "potion",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "potion",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "snowball",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "eye_of_ender",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "falling_block",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "spawn_position",
            typ: Type::BlockPos(0, 0, 0),
        }],
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
    },
    Class {
        name: "arrow",
        inherit: Some(&ABSTRACT_ARROW),
        fields: &[Field {
            name: "color",
            typ: Type::VarInt(-1), // TODO: custom type
        }],
    },
    Class {
        name: "spectral_arrow",
        inherit: Some(&ABSTRACT_ARROW),
        fields: &[],
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
    },
    Class {
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
                name: "typ",
                typ: Type::BoatVariant,
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
    },
    Class {
        name: "dragon_fireball",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
    },
    Class {
        name: "small_fireball",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "fireball",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "wither_skull",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "invulnerable",
            typ: Type::Bool(false),
        }],
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
    },
    Class {
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
    },
    Class {
        // TODO: How is glow item frame defined? This is a guess.
        name: "glow_item_frame",
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
        ],
    },
    Class {
        name: "armor_stand",
        inherit: Some(&LIVING_ENTITY),
        fields: &[
            Field {
                name: "armor_stand_bits",
                typ: Type::BitFields(&[
                    BitField {
                        name: "small",
                        offset: 0,
                        default: false,
                    },
                    BitField {
                        name: "has_arms",
                        offset: 1,
                        default: false,
                    },
                    BitField {
                        name: "no_baseplate",
                        offset: 2,
                        default: false,
                    },
                    BitField {
                        name: "is_marker",
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
    },
    Class {
        name: "bat",
        inherit: Some(&AMBIENT_CREATURE),
        fields: &[Field {
            name: "bat_bits",
            typ: Type::BitFields(&[BitField {
                name: "hanging",
                offset: 0,
                default: false,
            }]),
        }],
    },
    Class {
        name: "squid",
        inherit: Some(&WATER_ANIMAL),
        fields: &[],
    },
    Class {
        // TODO: How is glow squid defined? This is a guess.
        name: "glow_squid",
        inherit: Some(&WATER_ANIMAL),
        fields: &[],
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
    },
    Class {
        name: "cod",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[],
    },
    Class {
        name: "pufferfish",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[Field {
            name: "puff_state",
            typ: Type::VarInt(0), // TODO: PuffState in the range [0, 2]. (Bounded int?)
        }],
    },
    Class {
        name: "salmon",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[],
    },
    Class {
        name: "tropical_fish",
        inherit: Some(&ABSTRACT_FISH),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: TropicalFishVariant enum
        }],
    },
    Class {
        name: "horse",
        inherit: Some(&ABSTRACT_HORSE),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: HorseVariant enum
        }],
    },
    Class {
        name: "zombie_horse",
        inherit: Some(&ABSTRACT_HORSE),
        fields: &[],
    },
    Class {
        name: "skeleton_horse",
        inherit: Some(&ABSTRACT_HORSE),
        fields: &[],
    },
    Class {
        name: "donkey",
        inherit: Some(&CHESTED_HORSE),
        fields: &[],
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
    },
    Class {
        name: "trader_llama",
        inherit: None, // TODO: really?
        fields: &[],
    },
    Class {
        name: "mule",
        inherit: Some(&CHESTED_HORSE),
        fields: &[],
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
    },
    Class {
        name: "bee",
        inherit: Some(&ANIMAL),
        fields: &[
            Field {
                name: "bee_bits",
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
                name: "fox_bits",
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
    },
    Class {
        name: "ocelot",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "trusting",
            typ: Type::Bool(false),
        }],
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
                name: "panda_bits",
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
    },
    Class {
        name: "rabbit",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: rabbit variant enum.
        }],
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
    },
    Class {
        name: "polar_bear",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "standing_up",
            typ: Type::Bool(true),
        }],
    },
    Class {
        name: "chicken",
        inherit: Some(&ANIMAL),
        fields: &[],
    },
    COW,
    Class {
        name: "hoglin",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "zombification_immune",
            typ: Type::Bool(false),
        }],
    },
    Class {
        name: "mooshroom",
        inherit: Some(&COW),
        fields: &[Field {
            name: "variant",
            typ: Type::String("red"), // TODO: "red" or "brown" enum.
        }],
    },
    Class {
        name: "sheep",
        inherit: Some(&ANIMAL),
        fields: &[Field {
            name: "sheep_state",
            typ: Type::Byte(0), // TODO: sheep state type.
        }],
    },
    Class {
        name: "goat",
        inherit: Some(&ANIMAL),
        fields: &[], // TODO: What are the goat fields?
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
    },
    Class {
        name: "cat",
        inherit: Some(&TAMEABLE_ANIMAL),
        fields: &[
            Field {
                name: "variant",
                typ: Type::VarInt(1), // TODO: cat variant enum.
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
    },
    Class {
        name: "parrot",
        inherit: Some(&TAMEABLE_ANIMAL),
        fields: &[Field {
            name: "variant",
            typ: Type::VarInt(0), // TODO: parrot variant enum.
        }],
    },
    Class {
        name: "villager",
        inherit: Some(&ABSTRACT_VILLAGER),
        fields: &[Field {
            name: "villager_data",
            typ: Type::VillagerData,
        }],
    },
    Class {
        name: "wandering_trader",
        inherit: Some(&ABSTRACT_VILLAGER),
        fields: &[],
    },
    Class {
        name: "iron_golem",
        inherit: Some(&ABSTRACT_GOLEM),
        fields: &[Field {
            name: "iron_golem_bits",
            typ: Type::BitFields(&[BitField {
                name: "player_created",
                offset: 0,
                default: false,
            }]),
        }],
    },
    Class {
        name: "snow_golem",
        inherit: Some(&ABSTRACT_GOLEM),
        fields: &[Field {
            name: "snow_golem_bits",
            typ: Type::BitFields(&[BitField {
                name: "pumpkin_hat",
                offset: 4,
                default: true,
            }]),
        }],
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
    },
    Class {
        // TODO: how is this defined?
        name: "shulker_bullet",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
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
    },
    Class {
        name: "piglin_brute",
        inherit: Some(&BASE_PIGLIN),
        fields: &[],
    },
    Class {
        name: "blaze",
        inherit: Some(&MONSTER),
        fields: &[Field {
            name: "blaze_bits",
            typ: Type::BitFields(&[BitField {
                name: "blaze_on_fire", // TODO: better name for this?
                offset: 0,
                default: false,
            }]),
        }],
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
    },
    Class {
        name: "endermite",
        inherit: Some(&MONSTER),
        fields: &[],
    },
    Class {
        name: "giant",
        inherit: Some(&MONSTER),
        fields: &[],
    },
    GUARDIAN,
    Class {
        name: "elder_guardian",
        inherit: Some(&GUARDIAN),
        fields: &[],
    },
    Class {
        name: "silverfish",
        inherit: Some(&MONSTER),
        fields: &[],
    },
    Class {
        name: "vindicator",
        inherit: Some(&ABSTRACT_ILLAGER),
        fields: &[],
    },
    Class {
        name: "pillager",
        inherit: Some(&ABSTRACT_ILLAGER),
        fields: &[Field {
            name: "charging",
            typ: Type::Bool(false),
        }],
    },
    Class {
        name: "evoker",
        inherit: Some(&SPELLCASTER_ILLAGER),
        fields: &[],
    },
    Class {
        name: "illusioner",
        inherit: Some(&SPELLCASTER_ILLAGER),
        fields: &[],
    },
    Class {
        name: "ravager",
        inherit: Some(&RAIDER),
        fields: &[],
    },
    Class {
        name: "evoker_fangs",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
    },
    Class {
        name: "witch",
        inherit: Some(&RAIDER),
        fields: &[Field {
            name: "drinking_potion",
            typ: Type::Bool(false),
        }],
    },
    Class {
        name: "vex",
        inherit: Some(&MONSTER),
        fields: &[Field {
            name: "vex_bits",
            typ: Type::BitFields(&[BitField {
                name: "attacking",
                offset: 0,
                default: false,
            }]),
        }],
    },
    Class {
        name: "skeleton",
        inherit: Some(&ABSTRACT_SKELETON),
        fields: &[],
    },
    Class {
        name: "wither_skeleton",
        inherit: Some(&ABSTRACT_SKELETON),
        fields: &[],
    },
    Class {
        name: "stray",
        inherit: Some(&ABSTRACT_SKELETON),
        fields: &[],
    },
    SPIDER,
    Class {
        name: "cave_spider",
        inherit: Some(&SPIDER), // TODO: does cave_spider inherit from spider?
        fields: &[],
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
    },
    Class {
        name: "zoglin",
        inherit: Some(&MONSTER),
        fields: &[Field {
            name: "baby",
            typ: Type::Bool(false),
        }],
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
    },
    Class {
        name: "husk",
        inherit: Some(&ZOMBIE),
        fields: &[],
    },
    Class {
        name: "drowned",
        inherit: Some(&ZOMBIE),
        fields: &[],
    },
    Class {
        name: "zombified_piglin",
        inherit: Some(&ZOMBIE),
        fields: &[],
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
    },
    Class {
        name: "ender_dragon",
        inherit: Some(&MOB),
        fields: &[Field {
            name: "phase",
            typ: Type::VarInt(10), // TODO: dragon phase enum
        }],
    },
    Class {
        name: "ghast",
        inherit: Some(&FLYING),
        fields: &[Field {
            name: "attacking",
            typ: Type::Bool(false),
        }],
    },
    Class {
        name: "phantom",
        inherit: Some(&FLYING),
        fields: &[Field {
            name: "size",
            typ: Type::VarInt(0),
        }],
    },
    Class {
        name: "slime",
        inherit: Some(&MOB),
        fields: &[Field {
            name: "size",
            typ: Type::VarInt(1), // TODO: bounds?
        }],
    },
    Class {
        name: "magma_cube",
        inherit: Some(&MOB),
        fields: &[], // TODO: what are the fields?
    },
    Class {
        name: "llama_spit",
        inherit: Some(&BASE_ENTITY),
        fields: &[],
    },
    Class {
        name: "minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
    },
    Class {
        name: "hopper_minecart",
        inherit: Some(&ABSTRACT_MINECART_CONTAINER),
        fields: &[],
    },
    Class {
        name: "chest_minecart",
        inherit: Some(&ABSTRACT_MINECART_CONTAINER),
        fields: &[],
    },
    Class {
        name: "furnace_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[Field {
            name: "has_fuel",
            typ: Type::Bool(false),
        }],
    },
    Class {
        name: "tnt_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
    },
    Class {
        name: "spawner_minecart",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
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
    },
    Class {
        name: "tnt",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "fuse_timer",
            typ: Type::VarInt(80),
        }],
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

    let entity_type_variants = entities
        .iter()
        .map(|c| ident(c.name.to_pascal_case()))
        .collect::<Vec<_>>();

    let entity_structs = entities.iter().map(|&class| {
       let mut fields = Vec::new();
       collect_class_fields(class, &mut fields);

       let name = ident(class.name.to_pascal_case());
       let struct_fields = fields.iter().map(|&f| {
           let name = ident(f.name.to_snake_case());
           let typ = match f.typ {
               Type::BitFields(_) => quote! { u8 },
               Type::Byte(_) => quote! { u8 },
               Type::VarInt(_) => quote! { i32 },
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
               Type::OptVarInt => quote! { OptVarInt },
               Type::Pose => quote! { Pose },
               Type::OptEntityId => quote! { Option<EntityId> },
               Type::BoatVariant => quote! { BoatVariant },
               Type::MainHand => quote! { MainHand },
           };
           quote! {
               #name: #typ,
           }
       });

       let constructor_fields = fields.iter().map(|field| {
           let name = ident(field.name.to_snake_case());
           let val = match field.typ {
               Type::BitFields(bfs) => {
                   let mut default = 0;
                   for bf in bfs {
                       default = (bf.default as u8) << bf.offset;
                   }
                   quote! { #default }
               }
               Type::Byte(d) => quote! { #d },
               Type::VarInt(d) => quote! { #d },
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
               Type::OptVarInt => quote! { 0 },
               Type::Pose => quote! { Pose::default() },
               Type::OptEntityId => quote! { None },
               Type::BoatVariant => quote! { BoatVariant::default() },
               Type::MainHand => quote! { MainHand::default() },
           };

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
                            self.modified_bits |= 1 << #field_offset;
                        }

                        self.#name = #name;
                    }
                };

                match field.typ {
                    Type::BitFields(bfs) => bfs
                        .iter()
                        .map(|bf| {
                            if bf.name.to_snake_case().is_empty() {
                                eprintln!("{}", field.name);
                            }
                            let bit_name = ident(bf.name.to_snake_case());

                            let getter_name = ident(format!("get_{}", bit_name.to_string()));
                            let setter_name = ident(format!("set_{}", bit_name.to_string()));

                            let offset = bf.offset;

                            quote! {
                                pub fn #getter_name(&self) -> bool {
                                    (self.#name >> #offset) & 1 == 1
                                }

                                pub fn #setter_name(&mut self, #bit_name: bool) {
                                    let orig = self.#getter_name();

                                    self.#name = (self.#name & !(1 << #offset)) | ((#bit_name as u8) << #offset);

                                    if orig != self.#getter_name() {
                                        self.modified_bits |= 1 << #field_offset;
                                    }
                                }
                            }
                        })
                        .collect(),
                    Type::Byte(_) => standard_getter_setter(quote!(u8)),
                    Type::VarInt(_) => standard_getter_setter(quote!(i32)),
                    Type::Float(_) => standard_getter_setter(quote!(f32)),
                    Type::String(_) => quote! {
                        pub fn #getter_name(&self) -> &str {
                            &self.#name
                        }

                        pub fn #setter_name(&mut self, #name: impl Into<Box<str>>) {
                            let #name = #name.into();

                            if self.#name != #name {
                                self.modified_bits |= 1 << #field_offset;
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
                                self.modified_bits |= 1 << #field_offset;
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
                                self.modified_bits |= 1 << #field_offset;
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
                                self.modified_bits |= 1 << #field_offset;
                            }

                            self.#name = #name;
                        }
                    },
                    Type::Particle => quote! {}, // TODO
                    Type::VillagerData => standard_getter_setter(quote!(VillagerData)),
                    Type::OptVarInt => quote! {
                        pub fn #getter_name(&self) -> i32 {
                            self.#name.0
                        }

                        pub fn #setter_name(&mut self, #name: i32) {
                            if self.#name.0 != #name {
                                self.modified_bits |= 1 << #field_offset;
                            }

                            self.#name = OptVarInt(#name);
                        }
                    },
                    Type::Pose => standard_getter_setter(quote!(Pose)),
                    Type::OptEntityId => quote! {}, // TODO
                    Type::BoatVariant => standard_getter_setter(quote!(BoatVariant)),
                    Type::MainHand => standard_getter_setter(quote!(MainHand)),
                }
            })
            .collect::<TokenStream>();

        quote! {
            pub struct #name {
                /// Contains a set bit for each modified field.
                modified_bits: u32,
                #(#struct_fields)*
            }

            impl #name {
                pub(super) fn new() -> Self {
                    Self {
                        modified_bits: 0,
                        #(#constructor_fields)*
                    }
                }

                #getter_setters
            }
        }
    });

    let finished = quote! {
        pub enum EntityData {
            #(#entity_type_variants(#entity_type_variants),)*
        }

        impl EntityData {
            pub(super) fn new() -> Self {
                Self::Marker(Marker::new())
            }

            pub fn typ(&self) -> EntityType {
                match self {
                    #(Self::#entity_type_variants(_) => EntityType::#entity_type_variants,)*
                }
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum EntityType {
            #(#entity_type_variants,)*
        }

        impl Default for EntityType {
            fn default() -> Self {
                Self::Marker
            }
        }

        #(#entity_structs)*
    };

    write_to_out_path("entity.rs", &finished.to_string())
}

fn collect_class_fields(class: &Class, fields: &mut Vec<&'static Field>) {
    if let Some(parent) = class.inherit {
        collect_class_fields(parent, fields);
    }
    fields.extend(class.fields);
}
