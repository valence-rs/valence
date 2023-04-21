// TODO: Make a `Hitbox` component and plugin.

/*
/// Returns the hitbox of this entity.
///
/// The hitbox describes the space that an entity occupies. Clients interact
/// with this space to create an [interact event].
///
/// The hitbox of an entity is determined by its position, entity type, and
/// other state specific to that type.
///
/// [interact event]: crate::client::event::PlayerInteract
pub fn hitbox(&self) -> Aabb {
    fn baby(is_baby: bool, adult_hitbox: [f64; 3]) -> [f64; 3] {
        if is_baby {
            adult_hitbox.map(|a| a / 2.0)
        } else {
            adult_hitbox
        }
    }

    fn item_frame(pos: DVec3, rotation: i32) -> Aabb {
        let mut center_pos = pos + 0.5;

        match rotation {
            0 => center_pos.y += 0.46875,
            1 => center_pos.y -= 0.46875,
            2 => center_pos.z += 0.46875,
            3 => center_pos.z -= 0.46875,
            4 => center_pos.x += 0.46875,
            5 => center_pos.x -= 0.46875,
            _ => center_pos.y -= 0.46875,
        };

        let bounds = DVec3::from(match rotation {
            0 | 1 => [0.75, 0.0625, 0.75],
            2 | 3 => [0.75, 0.75, 0.0625],
            4 | 5 => [0.0625, 0.75, 0.75],
            _ => [0.75, 0.0625, 0.75],
        });

        Aabb {
            min: center_pos - bounds / 2.0,
            max: center_pos + bounds / 2.0,
        }
    }

    let dimensions = match &self.data {
        TrackedData::Allay(_) => [0.6, 0.35, 0.6],
        TrackedData::ChestBoat(_) => [1.375, 0.5625, 1.375],
        TrackedData::Frog(_) => [0.5, 0.5, 0.5],
        TrackedData::Tadpole(_) => [0.4, 0.3, 0.4],
        TrackedData::Warden(e) => match e.get_pose() {
            Pose::Emerging | Pose::Digging => [0.9, 1.0, 0.9],
            _ => [0.9, 2.9, 0.9],
        },
        TrackedData::AreaEffectCloud(e) => [
            e.get_radius() as f64 * 2.0,
            0.5,
            e.get_radius() as f64 * 2.0,
        ],
        TrackedData::ArmorStand(e) => {
            if e.get_marker() {
                [0.0, 0.0, 0.0]
            } else if e.get_small() {
                [0.5, 0.9875, 0.5]
            } else {
                [0.5, 1.975, 0.5]
            }
        }
        TrackedData::Arrow(_) => [0.5, 0.5, 0.5],
        TrackedData::Axolotl(_) => [1.3, 0.6, 1.3],
        TrackedData::Bat(_) => [0.5, 0.9, 0.5],
        TrackedData::Bee(e) => baby(e.get_child(), [0.7, 0.6, 0.7]),
        TrackedData::Blaze(_) => [0.6, 1.8, 0.6],
        TrackedData::Boat(_) => [1.375, 0.5625, 1.375],
        TrackedData::Camel(e) => baby(e.get_child(), [1.7, 2.375, 1.7]),
        TrackedData::Cat(_) => [0.6, 0.7, 0.6],
        TrackedData::CaveSpider(_) => [0.7, 0.5, 0.7],
        TrackedData::Chicken(e) => baby(e.get_child(), [0.4, 0.7, 0.4]),
        TrackedData::Cod(_) => [0.5, 0.3, 0.5],
        TrackedData::Cow(e) => baby(e.get_child(), [0.9, 1.4, 0.9]),
        TrackedData::Creeper(_) => [0.6, 1.7, 0.6],
        TrackedData::Dolphin(_) => [0.9, 0.6, 0.9],
        TrackedData::Donkey(e) => baby(e.get_child(), [1.5, 1.39648, 1.5]),
        TrackedData::DragonFireball(_) => [1.0, 1.0, 1.0],
        TrackedData::Drowned(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
        TrackedData::ElderGuardian(_) => [1.9975, 1.9975, 1.9975],
        TrackedData::EndCrystal(_) => [2.0, 2.0, 2.0],
        TrackedData::EnderDragon(_) => [16.0, 8.0, 16.0],
        TrackedData::Enderman(_) => [0.6, 2.9, 0.6],
        TrackedData::Endermite(_) => [0.4, 0.3, 0.4],
        TrackedData::Evoker(_) => [0.6, 1.95, 0.6],
        TrackedData::EvokerFangs(_) => [0.5, 0.8, 0.5],
        TrackedData::ExperienceOrb(_) => [0.5, 0.5, 0.5],
        TrackedData::EyeOfEnder(_) => [0.25, 0.25, 0.25],
        TrackedData::FallingBlock(_) => [0.98, 0.98, 0.98],
        TrackedData::FireworkRocket(_) => [0.25, 0.25, 0.25],
        TrackedData::Fox(e) => baby(e.get_child(), [0.6, 0.7, 0.6]),
        TrackedData::Ghast(_) => [4.0, 4.0, 4.0],
        TrackedData::Giant(_) => [3.6, 12.0, 3.6],
        TrackedData::GlowItemFrame(e) => return item_frame(self.position, e.get_rotation()),
        TrackedData::GlowSquid(_) => [0.8, 0.8, 0.8],
        TrackedData::Goat(e) => {
            if e.get_pose() == Pose::LongJumping {
                baby(e.get_child(), [0.63, 0.91, 0.63])
            } else {
                baby(e.get_child(), [0.9, 1.3, 0.9])
            }
        }
        TrackedData::Guardian(_) => [0.85, 0.85, 0.85],
        TrackedData::Hoglin(e) => baby(e.get_child(), [1.39648, 1.4, 1.39648]),
        TrackedData::Horse(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
        TrackedData::Husk(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
        TrackedData::Illusioner(_) => [0.6, 1.95, 0.6],
        TrackedData::IronGolem(_) => [1.4, 2.7, 1.4],
        TrackedData::Item(_) => [0.25, 0.25, 0.25],
        TrackedData::ItemFrame(e) => return item_frame(self.position, e.get_rotation()),
        TrackedData::Fireball(_) => [1.0, 1.0, 1.0],
        TrackedData::LeashKnot(_) => [0.375, 0.5, 0.375],
        TrackedData::Lightning(_) => [0.0, 0.0, 0.0],
        TrackedData::Llama(e) => baby(e.get_child(), [0.9, 1.87, 0.9]),
        TrackedData::LlamaSpit(_) => [0.25, 0.25, 0.25],
        TrackedData::MagmaCube(e) => {
            let s = 0.5202 * e.get_slime_size() as f64;
            [s, s, s]
        }
        TrackedData::Marker(_) => [0.0, 0.0, 0.0],
        TrackedData::Minecart(_) => [0.98, 0.7, 0.98],
        TrackedData::ChestMinecart(_) => [0.98, 0.7, 0.98],
        TrackedData::CommandBlockMinecart(_) => [0.98, 0.7, 0.98],
        TrackedData::FurnaceMinecart(_) => [0.98, 0.7, 0.98],
        TrackedData::HopperMinecart(_) => [0.98, 0.7, 0.98],
        TrackedData::SpawnerMinecart(_) => [0.98, 0.7, 0.98],
        TrackedData::TntMinecart(_) => [0.98, 0.7, 0.98],
        TrackedData::Mule(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
        TrackedData::Mooshroom(e) => baby(e.get_child(), [0.9, 1.4, 0.9]),
        TrackedData::Ocelot(e) => baby(e.get_child(), [0.6, 0.7, 0.6]),
        TrackedData::Painting(e) => {
            let bounds: UVec3 = match e.get_variant() {
                PaintingKind::Kebab => [1, 1, 1],
                PaintingKind::Aztec => [1, 1, 1],
                PaintingKind::Alban => [1, 1, 1],
                PaintingKind::Aztec2 => [1, 1, 1],
                PaintingKind::Bomb => [1, 1, 1],
                PaintingKind::Plant => [1, 1, 1],
                PaintingKind::Wasteland => [1, 1, 1],
                PaintingKind::Pool => [2, 1, 2],
                PaintingKind::Courbet => [2, 1, 2],
                PaintingKind::Sea => [2, 1, 2],
                PaintingKind::Sunset => [2, 1, 2],
                PaintingKind::Creebet => [2, 1, 2],
                PaintingKind::Wanderer => [1, 2, 1],
                PaintingKind::Graham => [1, 2, 1],
                PaintingKind::Match => [2, 2, 2],
                PaintingKind::Bust => [2, 2, 2],
                PaintingKind::Stage => [2, 2, 2],
                PaintingKind::Void => [2, 2, 2],
                PaintingKind::SkullAndRoses => [2, 2, 2],
                PaintingKind::Wither => [2, 2, 2],
                PaintingKind::Fighters => [4, 2, 4],
                PaintingKind::Pointer => [4, 4, 4],
                PaintingKind::Pigscene => [4, 4, 4],
                PaintingKind::BurningSkull => [4, 4, 4],
                PaintingKind::Skeleton => [4, 3, 4],
                PaintingKind::Earth => [2, 2, 2],
                PaintingKind::Wind => [2, 2, 2],
                PaintingKind::Water => [2, 2, 2],
                PaintingKind::Fire => [2, 2, 2],
                PaintingKind::DonkeyKong => [4, 3, 4],
            }
            .into();

            let mut center_pos = self.position + 0.5;

            let (facing_x, facing_z, cc_facing_x, cc_facing_z) =
                match ((self.yaw + 45.0).rem_euclid(360.0) / 90.0) as u8 {
                    0 => (0, 1, 1, 0),   // South
                    1 => (-1, 0, 0, 1),  // West
                    2 => (0, -1, -1, 0), // North
                    _ => (1, 0, 0, -1),  // East
                };

            center_pos.x -= facing_x as f64 * 0.46875;
            center_pos.z -= facing_z as f64 * 0.46875;

            center_pos.x += cc_facing_x as f64 * if bounds.x % 2 == 0 { 0.5 } else { 0.0 };
            center_pos.y += if bounds.y % 2 == 0 { 0.5 } else { 0.0 };
            center_pos.z += cc_facing_z as f64 * if bounds.z % 2 == 0 { 0.5 } else { 0.0 };

            let bounds = match (facing_x, facing_z) {
                (1, 0) | (-1, 0) => DVec3::new(0.0625, bounds.y as f64, bounds.z as f64),
                _ => DVec3::new(bounds.x as f64, bounds.y as f64, 0.0625),
            };

            return Aabb {
                min: center_pos - bounds / 2.0,
                max: center_pos + bounds / 2.0,
            };
        }
        TrackedData::Panda(e) => baby(e.get_child(), [1.3, 1.25, 1.3]),
        TrackedData::Parrot(_) => [0.5, 0.9, 0.5],
        TrackedData::Phantom(_) => [0.9, 0.5, 0.9],
        TrackedData::Pig(e) => baby(e.get_child(), [0.9, 0.9, 0.9]),
        TrackedData::Piglin(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
        TrackedData::PiglinBrute(_) => [0.6, 1.95, 0.6],
        TrackedData::Pillager(_) => [0.6, 1.95, 0.6],
        TrackedData::PolarBear(e) => baby(e.get_child(), [1.4, 1.4, 1.4]),
        TrackedData::Tnt(_) => [0.98, 0.98, 0.98],
        TrackedData::Pufferfish(_) => [0.7, 0.7, 0.7],
        TrackedData::Rabbit(e) => baby(e.get_child(), [0.4, 0.5, 0.4]),
        TrackedData::Ravager(_) => [1.95, 2.2, 1.95],
        TrackedData::Salmon(_) => [0.7, 0.4, 0.7],
        TrackedData::Sheep(e) => baby(e.get_child(), [0.9, 1.3, 0.9]),
        TrackedData::Shulker(e) => {
            const PI: f64 = std::f64::consts::PI;

            let pos = self.position + 0.5;
            let mut min = pos - 0.5;
            let mut max = pos + 0.5;

            let peek = 0.5 - f64::cos(e.get_peek_amount() as f64 * 0.01 * PI) * 0.5;

            match e.get_attached_face() {
                Facing::Down => max.y += peek,
                Facing::Up => min.y -= peek,
                Facing::North => max.z += peek,
                Facing::South => min.z -= peek,
                Facing::West => max.x += peek,
                Facing::East => min.x -= peek,
            }

            return Aabb { min, max };
        }
        TrackedData::ShulkerBullet(_) => [0.3125, 0.3125, 0.3125],
        TrackedData::Silverfish(_) => [0.4, 0.3, 0.4],
        TrackedData::Skeleton(_) => [0.6, 1.99, 0.6],
        TrackedData::SkeletonHorse(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
        TrackedData::Slime(e) => {
            let s = 0.5202 * e.get_slime_size() as f64;
            [s, s, s]
        }
        TrackedData::SmallFireball(_) => [0.3125, 0.3125, 0.3125],
        TrackedData::SnowGolem(_) => [0.7, 1.9, 0.7],
        TrackedData::Snowball(_) => [0.25, 0.25, 0.25],
        TrackedData::SpectralArrow(_) => [0.5, 0.5, 0.5],
        TrackedData::Spider(_) => [1.4, 0.9, 1.4],
        TrackedData::Squid(_) => [0.8, 0.8, 0.8],
        TrackedData::Stray(_) => [0.6, 1.99, 0.6],
        TrackedData::Strider(e) => baby(e.get_child(), [0.9, 1.7, 0.9]),
        TrackedData::Egg(_) => [0.25, 0.25, 0.25],
        TrackedData::EnderPearl(_) => [0.25, 0.25, 0.25],
        TrackedData::ExperienceBottle(_) => [0.25, 0.25, 0.25],
        TrackedData::Potion(_) => [0.25, 0.25, 0.25],
        TrackedData::Trident(_) => [0.5, 0.5, 0.5],
        TrackedData::TraderLlama(_) => [0.9, 1.87, 0.9],
        TrackedData::TropicalFish(_) => [0.5, 0.4, 0.5],
        TrackedData::Turtle(e) => {
            if e.get_child() {
                [0.36, 0.12, 0.36]
            } else {
                [1.2, 0.4, 1.2]
            }
        }
        TrackedData::Vex(_) => [0.4, 0.8, 0.4],
        TrackedData::Villager(e) => baby(e.get_child(), [0.6, 1.95, 0.6]),
        TrackedData::Vindicator(_) => [0.6, 1.95, 0.6],
        TrackedData::WanderingTrader(_) => [0.6, 1.95, 0.6],
        TrackedData::Witch(_) => [0.6, 1.95, 0.6],
        TrackedData::Wither(_) => [0.9, 3.5, 0.9],
        TrackedData::WitherSkeleton(_) => [0.7, 2.4, 0.7],
        TrackedData::WitherSkull(_) => [0.3125, 0.3125, 0.3125],
        TrackedData::Wolf(e) => baby(e.get_child(), [0.6, 0.85, 0.6]),
        TrackedData::Zoglin(e) => baby(e.get_baby(), [1.39648, 1.4, 1.39648]),
        TrackedData::Zombie(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
        TrackedData::ZombieHorse(e) => baby(e.get_child(), [1.39648, 1.6, 1.39648]),
        TrackedData::ZombieVillager(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
        TrackedData::ZombifiedPiglin(e) => baby(e.get_baby(), [0.6, 1.95, 0.6]),
        TrackedData::Player(e) => match e.get_pose() {
            Pose::Standing => [0.6, 1.8, 0.6],
            Pose::Sleeping => [0.2, 0.2, 0.2],
            Pose::FallFlying => [0.6, 0.6, 0.6],
            Pose::Swimming => [0.6, 0.6, 0.6],
            Pose::SpinAttack => [0.6, 0.6, 0.6],
            Pose::Sneaking => [0.6, 1.5, 0.6],
            Pose::Dying => [0.2, 0.2, 0.2],
            _ => [0.6, 1.8, 0.6],
        },
        TrackedData::FishingBobber(_) => [0.25, 0.25, 0.25],
    };

    Aabb::from_bottom_size(self.position, dimensions)
}
*/