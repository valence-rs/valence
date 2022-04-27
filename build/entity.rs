//! See: <https://wiki.vg/Entity_metadata>

#![allow(unused)]

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
    Rotations(f32, f32, f32),
    BlockPos(i32, i32, i32),
    OptPosition(Option<(i32, i32, i32)>),
    Direction(Direction),
    OptUuid,
    OptBlockId,
    Nbt,
    Particle,
    VillagerData,
    OptVarInt,
    Pose,
    OptBlockPos, // TODO: What is this type?
    // ==== Specialized ==== //
    OptEntityId,
    BoatType,
    MainHand,
}

enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

struct BitField {
    name: &'static str,
    offset: u8,
}

const BASE_ENTITY: Class = Class {
    name: "entity_base",
    inherit: None,
    fields: &[
        Field {
            name: "entity_base_bits",
            typ: Type::BitFields(&[
                BitField {
                    name: "on_fire",
                    offset: 0,
                },
                BitField {
                    name: "crouching",
                    offset: 1,
                },
                BitField {
                    name: "sprinting",
                    offset: 3, // Skipping unused field
                },
                BitField {
                    name: "swimming",
                    offset: 4,
                },
                BitField {
                    name: "invisible",
                    offset: 5,
                },
                BitField {
                    name: "glowing",
                    offset: 6,
                },
                BitField {
                    name: "elytra_flying",
                    offset: 7,
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
                },
                BitField {
                    name: "noclip",
                    offset: 1,
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
                },
                BitField {
                    name: "active_hand",
                    offset: 1,
                },
                BitField {
                    name: "riptide_spin_attack",
                    offset: 2,
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
            typ: Type::OptBlockPos,
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
            },
            BitField {
                name: "left_handed",
                offset: 1,
            },
            BitField {
                name: "aggressive",
                offset: 2,
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
                },
                BitField {
                    name: "saddled",
                    offset: 2,
                },
                BitField {
                    name: "bred",
                    offset: 3,
                },
                BitField {
                    name: "eating",
                    offset: 4,
                },
                BitField {
                    name: "rearing",
                    offset: 5,
                },
                BitField {
                    name: "mouth_open",
                    offset: 6,
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
            },
            BitField {
                name: "tamed",
                offset: 2, // Skip unused.
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
        name: "thrown_egg",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "thrown_ender_pearl",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "thrown_experience_bottle",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "item",
            typ: Type::Slot,
        }],
    },
    Class {
        name: "thrown_potion",
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
        name: "fishing_hook",
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
        name: "thrown_trident",
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
                typ: Type::BoatType,
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
                typ: Type::OptBlockPos,
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
                    },
                    BitField {
                        name: "jacket_enabled",
                        offset: 1,
                    },
                    BitField {
                        name: "left_sleeve_enabled",
                        offset: 2,
                    },
                    BitField {
                        name: "right_sleeve_enabled",
                        offset: 3,
                    },
                    BitField {
                        name: "left_pants_leg_enabled",
                        offset: 4,
                    },
                    BitField {
                        name: "right_pants_leg_enabled",
                        offset: 5,
                    },
                    BitField {
                        name: "hat_enabled",
                        offset: 6,
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
                    },
                    BitField {
                        name: "has_arms",
                        offset: 1,
                    },
                    BitField {
                        name: "no_baseplate",
                        offset: 2,
                    },
                    BitField {
                        name: "is_marker",
                        offset: 3,
                    },
                ]),
            },
            Field {
                name: "head_rotation",
                typ: Type::Rotations(0.0, 0.0, 0.0),
            },
            Field {
                name: "body_rotation",
                typ: Type::Rotations(0.0, 0.0, 0.0),
            },
            Field {
                name: "left_arm_rotation",
                typ: Type::Rotations(-10.0, 0.0, -10.0),
            },
            Field {
                name: "right_arm_rotation",
                typ: Type::Rotations(-15.0, 0.0, -10.0),
            },
            Field {
                name: "left_leg_rotation",
                typ: Type::Rotations(-1.0, 0.0, -1.0),
            },
            Field {
                name: "right_leg_rotation",
                typ: Type::Rotations(1.0, 0.0, 1.0),
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
        name: "puffer_fish",
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
                    },
                    BitField {
                        name: "stung",
                        offset: 2,
                    },
                    BitField {
                        name: "nectar",
                        offset: 3,
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
                    },
                    BitField {
                        name: "",
                        offset: 2, // Skip unused
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
                    },
                    BitField {
                        name: "rolling",
                        offset: 2,
                    },
                    BitField {
                        name: "sitting",
                        offset: 3,
                    },
                    BitField {
                        name: "on_back",
                        offset: 4,
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
                // TODO: should default to true.
            }]),
        }],
    },
    Class {
        name: "shulker",
        inherit: Some(&ABSTRACT_GOLEM),
        fields: &[
            Field {
                name: "attach_face",
                typ: Type::Direction(Direction::Down),
            },
            Field {
                name: "attachment_position",
                typ: Type::OptPosition(None),
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
        name: "evoker_fanges",
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
                typ: Type::OptBlockId,
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
        name: "minecart_hopper",
        inherit: Some(&ABSTRACT_MINECART_CONTAINER),
        fields: &[],
    },
    Class {
        name: "minecart_chest",
        inherit: Some(&ABSTRACT_MINECART_CONTAINER),
        fields: &[],
    },
    Class {
        name: "minecart_furnace",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[Field {
            name: "has_fuel",
            typ: Type::Bool(false),
        }],
    },
    Class {
        name: "minecart_tnt",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
    },
    Class {
        name: "minecart_spawner",
        inherit: Some(&ABSTRACT_MINECART),
        fields: &[],
    },
    Class {
        name: "minecart_command_block",
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
        name: "primed_tnt",
        inherit: Some(&BASE_ENTITY),
        fields: &[Field {
            name: "fuse_timer",
            typ: Type::VarInt(80),
        }],
    },
];

pub fn build() -> anyhow::Result<()> {
    Ok(())
}
