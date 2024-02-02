use super::*;

// TODO: should `set_if_neq` behavior be the default behavior for setters?
macro_rules! flags {
    (
        $(
            $component:path {
                $($flag:ident: $offset:literal),* $(,)?
            }
        )*

    ) => {
        $(
            impl $component {
                $(
                    #[doc = "Gets the bit at offset "]
                    #[doc = stringify!($offset)]
                    #[doc = "."]
                    #[inline]
                    pub const fn $flag(&self) -> bool {
                        (self.0 >> $offset) & 1 == 1
                    }

                    paste! {
                        #[doc = "Sets the bit at offset "]
                        #[doc = stringify!($offset)]
                        #[doc = "."]
                        #[inline]
                        pub fn [< set_$flag >] (&mut self, $flag: bool) {
                            self.0 = (self.0 & !(1 << $offset)) | (($flag as i8) << $offset);
                        }
                    }
                )*
            }
        )*
    }
}

flags! {
    entity::Flags {
        on_fire: 0,
        sneaking: 1,
        sprinting: 3,
        swimming: 4,
        invisible: 5,
        glowing: 6,
        fall_flying: 7,
    }
    persistent_projectile::ProjectileFlags {
        critical: 0,
        no_clip: 1,
    }
    living::LivingFlags {
        using_item: 0,
        off_hand_active: 1,
        using_riptide: 2,
    }
    player::PlayerModelParts {
        cape: 0,
        jacket: 1,
        left_sleeve: 2,
        right_sleeve: 3,
        left_pants_leg: 4,
        right_pants_leg: 5,
        hat: 6,
    }
    player::MainArm {
        right: 0,
    }
    armor_stand::ArmorStandFlags {
        small: 0,
        show_arms: 1,
        hide_base_plate: 2,
        marker: 3,
    }
    mob::MobFlags {
        ai_disabled: 0,
        left_handed: 1,
        attacking: 2,
    }
    bat::BatFlags {
        hanging: 0,
    }
    abstract_horse::HorseFlags {
        tamed: 1,
        saddled: 2,
        bred: 3,
        eating_grass: 4,
        angry: 5,
        eating: 6,
    }
    fox::FoxFlags {
        sitting: 0,
        crouching: 2,
        rolling_head: 3,
        chasing: 4,
        sleeping: 5,
        walking: 6,
        aggressive: 7,
    }
    panda::PandaFlags {
        sneezing: 1,
        playing: 2,
        sitting: 3,
        lying_on_back: 4,
    }
    tameable::TameableFlags {
        sitting_pose: 0,
        tamed: 2,
    }
    iron_golem::IronGolemFlags {
        player_created: 0,
    }
    snow_golem::SnowGolemFlags {
        has_pumpkin: 4,
    }
    blaze::BlazeFlags {
        fire_active: 0,
    }
    vex::VexFlags {
        charging: 0,
    }
    spider::SpiderFlags {
        climbing_wall: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_flags() {
        let mut flags = entity::Flags(0);

        flags.set_on_fire(true);
        let before = flags.clone();
        assert_ne!(flags.0, 0);
        flags.set_on_fire(true);
        assert_eq!(before, flags);
        flags.set_on_fire(false);
        assert_eq!(flags.0, 0);
    }
}
