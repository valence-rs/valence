use bevy_ecs::{
    prelude::*,
    system::{Command, SystemState},
};

use valence_entity::{hitbox::Hitbox, living::LivingEntity, Look};
use valence_generated::block::{PropName, PropValue};
use valence_math::{Aabb, DVec3};
use valence_protocol::{BlockKind, BlockPos, BlockState, Direction};

use crate::{interact_block::InteractBlockEvent, ChunkLayer};

//from https://minecraft.fandom.com/wiki/Block_states
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PlacementRule {
    Basic,
    //For blocks that don't necessarily have special props, but have special rules for what can be placed on top/side of it (put together from https://minecraft.fandom.com/wiki/Opacity/Placement)
    //Overlap or equality with the set of not opaque blocks?
    Special,
    Anvils,
    AmethystBudsAndAmethystCluster,
    Bamboo,
    Banners,
    Barrel,
    BasaltAndPolishedBasalt,
    Beds,
    Bedrock,
    Beehive,
    Beetroots,
    Bell,
    BigDripleaf,
    BlastFurnace,
    BlockOfBambooAndBlockOfStrippedBamboo,
    BoneBlock,
    Border,
    BrewingStand,
    BubbleColumn,
    Buttons,
    Cactus,
    Cake,
    Campfire,
    Candles,
    Carpets,
    Carrots,
    Cauldron,
    CaveVines,
    Chain,
    ChemistryTable,
    ChestAndTrappedChest,
    EnderChest,
    ChiseledBookshelf,
    ChorusFlower,
    ChorusPlant,
    Cocoa,
    CommandBlocks,
    Composter,
    Concrete,
    ConcretePowder,
    Conduit,
    Coral,
    CoralBlock,
    CoralFan,
    DaylightDetector,
    //Missing from the overview:
    DecoratedPot,
    Deepslate,
    //Bedrock Only: Dirt,
    DispenserAndDropper,
    Doors,
    EndPortalFrame,
    EndRod,
    Farmland,
    Fences,
    FenceGates,
    Fire,
    Flowers,
    FlowerPot,
    Froglight,
    FrostedIce,
    Furnace,
    //Bedrock Only: Glass,
    GlassPanes,
    GlazedTerracotta,
    GlowLichen,
    GrassBlockMyceliumAndPodzol,
    Grindstone,
    //Bedrock Only: HardenedGlass,
    //Bedrock Only: HardenedGlassPane,
    //Missing from the overview:
    HangingSign,
    HayBale,
    Hopper,
    InfestedBlock,
    IronBars,
    ItemFrameAndGlowItemFrame,
    JigsawBlock,
    JackOLantern,
    Jukebox,
    Kelp,
    Ladder,
    LanternAndSoulLantern,
    Leaves,
    Lectern,
    Lever,
    LightBlock,
    LightningRod,
    Logs,
    Loom,
    MangroveRoots,
    MelonStem,
    MobHeads,
    MuddyMangroveRoots,
    MushroomBlocks,
    NetherWart,
    NetherPortal,
    NoteBlock,
    Observer,
    PinkPetals,
    Pistons,
    MovingPiston,
    PistonHead,
    Planks,
    Potatoes,
    PointedDripstone,
    PressurePlates,
    Prismarine,
    PumpkinAndCarvedPumpkin,
    PumpkinStem,
    PurpurAndQuartzPillar,
    Rails,
    Rail,
    ActivatorRailAndDetectorRailAndPoweredRail,
    RedstoneComparator,
    RedstoneDust,
    RedstoneLamp,
    RedstoneOre,
    RedstoneRepeater,
    RedstoneTorch,
    RespawnAnchor,
    //Bedrock Only: SandAndRedSand,
    //Bedrock Only: SandstoneAndRedSandstone,
    Saplings,
    Scaffolding,
    SculkCatalyst,
    SculkSensor,
    SculkShrieker,
    SculkVein,
    SeaPickle,
    ShulkerBoxes,
    Sign,
    Slabs,
    SmallDripleaf,
    Smoker,
    //Missing from the overview:
    SnifferEgg,
    Snow,
    Sponge,
    Stairs,
    Stones,
    StoneBricks,
    Stonecutter,
    StructureBlock,
    StructureVoid,
    SugarCane,
    SweetBerryBush,
    TallGrassAndLargeFern,
    TallSeagrass,
    Target,
    Terracottas,
    TNT,
    TorchAndSoulTorch,
    Trapdoors,
    Tripwire,
    TripwireHook,
    TurtleEgg,
    TwistingVines,
    UnderwaterTorch,
    Vines,
    Walls,
    WeepingVines,
    WheatCrop,
    Wood,
    Water,
    Lava,
}

#[derive(Clone)]
pub enum CardinalDirection {
    North,
    West,
    South,
    East,
}

impl CardinalDirection {
    pub const fn into_prop_value(self) -> PropValue {
        match self {
            Self::North => PropValue::North,
            Self::West => PropValue::West,
            Self::South => PropValue::South,
            Self::East => PropValue::East,
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::West => Self::East,
            Self::South => Self::North,
            Self::East => Self::West,
        }
    }
}

#[derive(Clone)]
pub enum Half {
    Bottom,
    Top,
}

impl Half {
    pub fn into_prop_value(self) -> PropValue {
        match self {
            Half::Bottom => PropValue::Bottom,
            Half::Top => PropValue::Top,
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Bottom => Self::Top,
            Self::Top => Self::Bottom,
        }
    }
}

#[derive(Clone)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn into_prop_value(self) -> PropValue {
        match self {
            Side::Left => PropValue::Left,
            Side::Right => PropValue::Right,
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl PlacementRule {
    //TODO: WIP
    pub const fn from_kind(kind: BlockKind) -> Self {
        match kind {
            BlockKind::OakStairs
            | BlockKind::CobblestoneStairs
            | BlockKind::BrickStairs
            | BlockKind::StoneBrickStairs
            | BlockKind::MudBrickStairs
            | BlockKind::NetherBrickStairs
            | BlockKind::SandstoneStairs
            | BlockKind::SpruceStairs
            | BlockKind::BirchStairs
            | BlockKind::JungleStairs
            | BlockKind::QuartzStairs
            | BlockKind::AcaciaStairs
            | BlockKind::CherryStairs
            | BlockKind::DarkOakStairs
            | BlockKind::MangroveStairs
            | BlockKind::BambooStairs
            | BlockKind::BambooMosaicStairs
            | BlockKind::PrismarineStairs
            | BlockKind::PrismarineBrickStairs
            | BlockKind::DarkPrismarineStairs
            | BlockKind::RedSandstoneStairs
            | BlockKind::PurpurStairs
            | BlockKind::PolishedGraniteStairs
            | BlockKind::SmoothRedSandstoneStairs
            | BlockKind::MossyStoneBrickStairs
            | BlockKind::PolishedDioriteStairs
            | BlockKind::MossyCobblestoneStairs
            | BlockKind::EndStoneBrickStairs
            | BlockKind::StoneStairs
            | BlockKind::SmoothSandstoneStairs
            | BlockKind::SmoothQuartzStairs
            | BlockKind::GraniteStairs
            | BlockKind::AndesiteStairs
            | BlockKind::RedNetherBrickStairs
            | BlockKind::PolishedAndesiteStairs
            | BlockKind::DioriteStairs
            | BlockKind::CrimsonStairs
            | BlockKind::WarpedStairs
            | BlockKind::BlackstoneStairs
            | BlockKind::PolishedBlackstoneBrickStairs
            | BlockKind::PolishedBlackstoneStairs
            | BlockKind::OxidizedCutCopperStairs
            | BlockKind::WeatheredCutCopperStairs
            | BlockKind::ExposedCutCopperStairs
            | BlockKind::CutCopperStairs
            | BlockKind::WaxedOxidizedCutCopperStairs
            | BlockKind::WaxedWeatheredCutCopperStairs
            | BlockKind::WaxedExposedCutCopperStairs
            | BlockKind::WaxedCutCopperStairs
            | BlockKind::CobbledDeepslateStairs
            | BlockKind::PolishedDeepslateStairs
            | BlockKind::DeepslateTileStairs
            | BlockKind::DeepslateBrickStairs => Self::Stairs,
            BlockKind::PrismarineSlab
            | BlockKind::PrismarineBrickSlab
            | BlockKind::DarkPrismarineSlab
            | BlockKind::OakSlab
            | BlockKind::SpruceSlab
            | BlockKind::BirchSlab
            | BlockKind::JungleSlab
            | BlockKind::AcaciaSlab
            | BlockKind::CherrySlab
            | BlockKind::DarkOakSlab
            | BlockKind::MangroveSlab
            | BlockKind::BambooSlab
            | BlockKind::BambooMosaicSlab
            | BlockKind::StoneSlab
            | BlockKind::SmoothStoneSlab
            | BlockKind::SandstoneSlab
            | BlockKind::CutSandstoneSlab
            | BlockKind::PetrifiedOakSlab
            | BlockKind::CobblestoneSlab
            | BlockKind::BrickSlab
            | BlockKind::StoneBrickSlab
            | BlockKind::MudBrickSlab
            | BlockKind::NetherBrickSlab
            | BlockKind::QuartzSlab
            | BlockKind::RedSandstoneSlab
            | BlockKind::CutRedSandstoneSlab
            | BlockKind::PurpurSlab
            | BlockKind::PolishedGraniteSlab
            | BlockKind::SmoothRedSandstoneSlab
            | BlockKind::MossyStoneBrickSlab
            | BlockKind::PolishedDioriteSlab
            | BlockKind::MossyCobblestoneSlab
            | BlockKind::EndStoneBrickSlab
            | BlockKind::SmoothSandstoneSlab
            | BlockKind::SmoothQuartzSlab
            | BlockKind::GraniteSlab
            | BlockKind::AndesiteSlab
            | BlockKind::RedNetherBrickSlab
            | BlockKind::PolishedAndesiteSlab
            | BlockKind::DioriteSlab
            | BlockKind::CrimsonSlab
            | BlockKind::WarpedSlab
            | BlockKind::BlackstoneSlab
            | BlockKind::PolishedBlackstoneBrickSlab
            | BlockKind::PolishedBlackstoneSlab
            | BlockKind::OxidizedCutCopperSlab
            | BlockKind::WeatheredCutCopperSlab
            | BlockKind::ExposedCutCopperSlab
            | BlockKind::CutCopperSlab
            | BlockKind::WaxedOxidizedCutCopperSlab
            | BlockKind::WaxedWeatheredCutCopperSlab
            | BlockKind::WaxedExposedCutCopperSlab
            | BlockKind::WaxedCutCopperSlab
            | BlockKind::CobbledDeepslateSlab
            | BlockKind::PolishedDeepslateSlab
            | BlockKind::DeepslateTileSlab
            | BlockKind::DeepslateBrickSlab => Self::Slabs,
            BlockKind::StoneButton
            | BlockKind::OakButton
            | BlockKind::SpruceButton
            | BlockKind::BirchButton
            | BlockKind::JungleButton
            | BlockKind::AcaciaButton
            | BlockKind::CherryButton
            | BlockKind::DarkOakButton
            | BlockKind::MangroveButton
            | BlockKind::BambooButton
            | BlockKind::CrimsonButton
            | BlockKind::WarpedButton => Self::Buttons,
            BlockKind::Lever => Self::Lever,
            BlockKind::OakDoor
            | BlockKind::IronDoor
            | BlockKind::SpruceDoor
            | BlockKind::BirchDoor
            | BlockKind::JungleDoor
            | BlockKind::AcaciaDoor
            | BlockKind::CherryDoor
            | BlockKind::DarkOakDoor
            | BlockKind::MangroveDoor
            | BlockKind::BambooDoor
            | BlockKind::CrimsonDoor
            | BlockKind::WarpedDoor => Self::Doors,
            BlockKind::Anvil | BlockKind::ChippedAnvil | BlockKind::DamagedAnvil => Self::Anvils,
            BlockKind::Bamboo | BlockKind::BambooSapling => Self::Bamboo,
            BlockKind::Bell => Self::Bell,
            BlockKind::BrewingStand => Self::BrewingStand,
            BlockKind::Cactus => Self::Cactus,
            BlockKind::Cauldron
            | BlockKind::LavaCauldron
            | BlockKind::WaterCauldron
            | BlockKind::PowderSnowCauldron => Self::Cauldron,
            BlockKind::Chain => Self::Chain,
            BlockKind::ChorusFlower => Self::ChorusFlower,
            BlockKind::ChorusPlant => Self::ChorusPlant,
            BlockKind::Composter => Self::Composter,
            BlockKind::DecoratedPot => Self::DecoratedPot,
            BlockKind::EndPortalFrame => Self::EndPortalFrame,
            BlockKind::EndRod => Self::EndRod,
            BlockKind::Farmland => Self::Farmland,
            BlockKind::OakFence
            | BlockKind::NetherBrickFence
            | BlockKind::SpruceFence
            | BlockKind::BirchFence
            | BlockKind::JungleFence
            | BlockKind::AcaciaFence
            | BlockKind::CherryFence
            | BlockKind::DarkOakFence
            | BlockKind::MangroveFence
            | BlockKind::BambooFence
            | BlockKind::CrimsonFence
            | BlockKind::WarpedFence => Self::Fences,
            BlockKind::OakFenceGate
            | BlockKind::SpruceFenceGate
            | BlockKind::BirchFenceGate
            | BlockKind::JungleFenceGate
            | BlockKind::AcaciaFenceGate
            | BlockKind::CherryFenceGate
            | BlockKind::DarkOakFenceGate
            | BlockKind::MangroveFenceGate
            | BlockKind::BambooFenceGate
            | BlockKind::CrimsonFenceGate
            | BlockKind::WarpedFenceGate => Self::FenceGates,
            BlockKind::GlassPane
            | BlockKind::WhiteStainedGlassPane
            | BlockKind::OrangeStainedGlassPane
            | BlockKind::MagentaStainedGlassPane
            | BlockKind::LightBlueStainedGlassPane
            | BlockKind::YellowStainedGlassPane
            | BlockKind::LimeStainedGlassPane
            | BlockKind::PinkStainedGlassPane
            | BlockKind::GrayStainedGlassPane
            | BlockKind::LightGrayStainedGlassPane
            | BlockKind::CyanStainedGlassPane
            | BlockKind::PurpleStainedGlassPane
            | BlockKind::BlueStainedGlassPane
            | BlockKind::BrownStainedGlassPane
            | BlockKind::GreenStainedGlassPane
            | BlockKind::RedStainedGlassPane
            | BlockKind::BlackStainedGlassPane => Self::GlassPanes,
            BlockKind::Grindstone => Self::Grindstone,
            BlockKind::OakHangingSign
            | BlockKind::SpruceHangingSign
            | BlockKind::BirchHangingSign
            | BlockKind::AcaciaHangingSign
            | BlockKind::CherryHangingSign
            | BlockKind::JungleHangingSign
            | BlockKind::DarkOakHangingSign
            | BlockKind::CrimsonHangingSign
            | BlockKind::WarpedHangingSign
            | BlockKind::MangroveHangingSign
            | BlockKind::BambooHangingSign
            | BlockKind::OakWallHangingSign
            | BlockKind::SpruceWallHangingSign
            | BlockKind::BirchWallHangingSign
            | BlockKind::AcaciaWallHangingSign
            | BlockKind::CherryWallHangingSign
            | BlockKind::JungleWallHangingSign
            | BlockKind::DarkOakWallHangingSign
            | BlockKind::MangroveWallHangingSign
            | BlockKind::CrimsonWallHangingSign
            | BlockKind::WarpedWallHangingSign
            | BlockKind::BambooWallHangingSign => Self::HangingSign,
            BlockKind::Hopper => Self::Hopper,
            BlockKind::FrostedIce => Self::FrostedIce,
            BlockKind::IronBars => Self::IronBars,
            BlockKind::Ladder => Self::Ladder,
            BlockKind::OakLeaves
            | BlockKind::SpruceLeaves
            | BlockKind::BirchLeaves
            | BlockKind::JungleLeaves
            | BlockKind::AcaciaLeaves
            | BlockKind::CherryLeaves
            | BlockKind::DarkOakLeaves
            | BlockKind::MangroveLeaves
            | BlockKind::AzaleaLeaves
            | BlockKind::FloweringAzaleaLeaves => Self::Leaves,
            BlockKind::Lectern => Self::Lectern,
            BlockKind::LightningRod => Self::LightningRod,
            BlockKind::Scaffolding => Self::Scaffolding,
            BlockKind::SnifferEgg => Self::SnifferEgg,
            BlockKind::Snow => Self::Snow,
            BlockKind::CobblestoneWall
            | BlockKind::MossyCobblestoneWall
            | BlockKind::BrickWall
            | BlockKind::PrismarineWall
            | BlockKind::RedSandstoneWall
            | BlockKind::MossyStoneBrickWall
            | BlockKind::GraniteWall
            | BlockKind::StoneBrickWall
            | BlockKind::MudBrickWall
            | BlockKind::NetherBrickWall
            | BlockKind::AndesiteWall
            | BlockKind::RedNetherBrickWall
            | BlockKind::SandstoneWall
            | BlockKind::EndStoneBrickWall
            | BlockKind::DioriteWall
            | BlockKind::BlackstoneWall
            | BlockKind::PolishedBlackstoneBrickWall
            | BlockKind::PolishedBlackstoneWall
            | BlockKind::CobbledDeepslateWall
            | BlockKind::PolishedDeepslateWall
            | BlockKind::DeepslateTileWall
            | BlockKind::DeepslateBrickWall => Self::Walls,
            BlockKind::WhiteCarpet
            | BlockKind::OrangeCarpet
            | BlockKind::MagentaCarpet
            | BlockKind::LightBlueCarpet
            | BlockKind::YellowCarpet
            | BlockKind::LimeCarpet
            | BlockKind::PinkCarpet
            | BlockKind::GrayCarpet
            | BlockKind::LightGrayCarpet
            | BlockKind::CyanCarpet
            | BlockKind::PurpleCarpet
            | BlockKind::BlueCarpet
            | BlockKind::BrownCarpet
            | BlockKind::GreenCarpet
            | BlockKind::RedCarpet
            | BlockKind::BlackCarpet
            | BlockKind::MossCarpet => Self::Carpets,
            BlockKind::FlowerPot => Self::FlowerPot,
            BlockKind::OakTrapdoor
            | BlockKind::SpruceTrapdoor
            | BlockKind::BirchTrapdoor
            | BlockKind::JungleTrapdoor
            | BlockKind::AcaciaTrapdoor
            | BlockKind::CherryTrapdoor
            | BlockKind::DarkOakTrapdoor
            | BlockKind::MangroveTrapdoor
            | BlockKind::BambooTrapdoor
            | BlockKind::IronTrapdoor
            | BlockKind::CrimsonTrapdoor
            | BlockKind::WarpedTrapdoor => Self::Trapdoors,
            BlockKind::Observer => Self::Observer,
            BlockKind::WhiteBed
            | BlockKind::OrangeBed
            | BlockKind::MagentaBed
            | BlockKind::LightBlueBed
            | BlockKind::YellowBed
            | BlockKind::LimeBed
            | BlockKind::PinkBed
            | BlockKind::GrayBed
            | BlockKind::LightGrayBed
            | BlockKind::CyanBed
            | BlockKind::PurpleBed
            | BlockKind::BlueBed
            | BlockKind::BrownBed
            | BlockKind::GreenBed
            | BlockKind::RedBed
            | BlockKind::BlackBed => Self::Beds,
            BlockKind::Chest | BlockKind::TrappedChest => Self::ChestAndTrappedChest,
            BlockKind::EnderChest => PlacementRule::EnderChest,

            BlockKind::Water => Self::Water,
            BlockKind::Lava => Self::Lava,
            BlockKind::Azalea
            | BlockKind::FloweringAzalea
            | BlockKind::Barrier
            | BlockKind::Cobweb
            | BlockKind::DirtPath
            | BlockKind::DragonEgg
            | BlockKind::HoneyBlock
            | BlockKind::Ice
            | BlockKind::BlueIce
            | BlockKind::PackedIce => Self::Special,
            _ => Self::Basic,
        }
    }

    pub const fn accumulates(self) -> bool {
        match self {
            Self::Candles | Self::PinkPetals | Self::SeaPickle | Self::Slabs | Self::TurtleEgg => {
                true
            }
            _ => false,
        }
    }
}

//TODO: WIP
//Based on https://minecraft.fandom.com/wiki/Opacity/Placement
//Plant mechanics not covered there
pub fn can_be_placed_on_top(this: BlockState, of: BlockState) -> bool {
    match PlacementRule::from_kind(of.to_kind()) {
        PlacementRule::Basic | PlacementRule::Observer => return true,
        PlacementRule::Trapdoors => {
            if of.get(PropName::Half).unwrap() == PropValue::Top
                && of.get(PropName::Open).unwrap() == PropValue::False
            {
                return true;
            }
        }
        PlacementRule::ChorusPlant => {
            if of.get(PropName::Up).unwrap() == PropValue::True {
                return false;
            }
        }
        _ => {}
    }

    match PlacementRule::from_kind(this.to_kind()) {
        PlacementRule::Carpets | PlacementRule::FlowerPot | PlacementRule::Grindstone => true,
        PlacementRule::Doors => match PlacementRule::from_kind(of.to_kind()) {
            PlacementRule::ChorusFlower
            | PlacementRule::FrostedIce
            | PlacementRule::Scaffolding => true,
            PlacementRule::Snow => of.get(PropName::Layers).unwrap() == PropValue::_8,
            _ => match of.to_kind() {
                BlockKind::Azalea
                | BlockKind::FloweringAzalea
                | BlockKind::Ice
                | BlockKind::BlueIce
                | BlockKind::PackedIce
                | BlockKind::FrostedIce => true,
                _ => false,
            },
        },
        _ => unimplemented!(),
    }
}

pub fn can_be_placed_on_side(this: BlockState, of: BlockState) -> bool {
    match PlacementRule::from_kind(of.to_kind()) {
        PlacementRule::Basic
        | PlacementRule::FrostedIce
        | PlacementRule::Doors
        | PlacementRule::Composter
        | PlacementRule::Observer => return true,
        PlacementRule::Trapdoors => {
            if of.get(PropName::Half).unwrap() == PropValue::Top
                && of.get(PropName::Open).unwrap() == PropValue::False
            {
                return true;
            }
        }
        PlacementRule::Scaffolding
        | PlacementRule::EndRod
        | PlacementRule::MobHeads
        | PlacementRule::ChorusPlant => {
            return false;
        }
        PlacementRule::Snow => match of.get(PropName::Layers).unwrap() {
            PropValue::_2
            | PropValue::_3
            | PropValue::_4
            | PropValue::_5
            | PropValue::_6
            | PropValue::_7 => return false,
            _ => {}
        },
        _ => {}
    }

    match of.to_kind() {
        BlockKind::Azalea | BlockKind::FloweringAzalea => {
            return false;
        }
        _ => {}
    }

    match PlacementRule::from_kind(this.to_kind()) {
        _ => unimplemented!(),
    }
}

//TODO: Builder
#[derive(Clone)]
pub struct PlaceBlockCommand {
    pub block_kind: BlockKind,
    pub interact_block_event: InteractBlockEvent,
    pub look: Look,
    pub ignore_collisions: bool,
}

impl Command for PlaceBlockCommand {
    fn apply(self, world: &mut bevy_ecs::world::World) {
        let Self {
            block_kind,
            interact_block_event:
                InteractBlockEvent {
                    client: _,
                    hand: _,
                    position: interact_pos,
                    face,
                    cursor_pos,
                    head_inside_block,
                    sequence: _,
                },
            look,
            ignore_collisions,
        } = self;

        let mut system_state =
            SystemState::<(Query<&mut ChunkLayer>, Query<&Hitbox, With<LivingEntity>>)>::new(world);
        let (mut layers, entity_hitboxes) = system_state.get_mut(world);

        let mut layer = layers.single_mut();

        let interact_block = layer.block(interact_pos).unwrap_or_default();

        let facing_pos = interact_pos.get_in_direction(face);
        let facing_block = layer.block(facing_pos).unwrap_or_default();

        let mut block = block_kind.to_state();
        let placement_rule = PlacementRule::from_kind(block_kind);

        let is_facing_block_replaceable: bool = facing_block.state.is_replaceable();
        let has_hitbox = block.collision_shapes().next().is_some();
        let was_interacted_inside = cursor_pos.x != 0.0
            && cursor_pos.x != 1.0
            && cursor_pos.y != 0.0
            && cursor_pos.y != 1.0
            && cursor_pos.z != 0.0
            && cursor_pos.z != 1.0;

        let mut new_block_pos = facing_pos;

        let is_accumulating = placement_rule.accumulates()
            && if was_interacted_inside {
                new_block_pos = interact_pos;
                block_kind == interact_block.state.to_kind()
            } else {
                block_kind == facing_block.state.to_kind()
            };
        let is_placing_water = placement_rule == PlacementRule::Water;
        // Not placeable if NOT one of these things:
        if !(is_facing_block_replaceable || is_accumulating || is_placing_water) {
            return;
        }

        //if collisions are not ignored && a solid block is placed
        if !ignore_collisions && has_hitbox {
            // If any entity is in the way of the block, don't place it.
            let bx = facing_pos.x as f64;
            let by = facing_pos.y as f64;
            let bz = facing_pos.z as f64;

            let block_aabb = Aabb::new(
                DVec3::new(bx, by, bz),
                DVec3::new(bx + 1.0, by + 1.0, bz + 1.0),
            );

            for hitbox in entity_hitboxes.iter() {
                let entity_aabb = hitbox.get();

                pub fn intersects(this: Aabb, other: Aabb) -> bool {
                    this.max().x > other.min().x
                        && other.max().x > this.min().x
                        && this.max().y > other.min().y
                        && other.max().y > this.min().y
                        && this.max().z > other.min().z
                        && other.max().z > this.min().z
                }
                // if the entity is in the way of the block, don't place it.
                if intersects(entity_aabb, block_aabb) {
                    return;
                }
            }
        }

        let direction = {
            let normalized_yaw = look.yaw.rem_euclid(360.0);
            if normalized_yaw >= 45.0 && normalized_yaw < 135.0 {
                CardinalDirection::West
            } else if normalized_yaw >= 135.0 && normalized_yaw < 225.0 {
                CardinalDirection::North
            } else if normalized_yaw >= 225.0 && normalized_yaw < 315.0 {
                CardinalDirection::East
            } else {
                CardinalDirection::South
            }
        };

        //if put on top of a block
        let half = if cursor_pos.y == 1.0 {
            Half::Bottom
        }
        //if put on the bottom of a block
        else if cursor_pos.y == 0.0 {
            Half::Top
        } else {
            //else depending on top/bottom part of block
            match cursor_pos.y.total_cmp(&0.5) {
                std::cmp::Ordering::Less => Half::Bottom,
                _ => Half::Top,
            }
        };

        let side = match direction {
            CardinalDirection::North => match cursor_pos.x.total_cmp(&0.5) {
                std::cmp::Ordering::Less => Side::Left,
                _ => Side::Right,
            },
            CardinalDirection::West => match cursor_pos.z.total_cmp(&0.5) {
                std::cmp::Ordering::Less => Side::Right,
                _ => Side::Left,
            },
            CardinalDirection::South => match cursor_pos.x.total_cmp(&0.5) {
                std::cmp::Ordering::Less => Side::Right,
                _ => Side::Left,
            },
            CardinalDirection::East => match cursor_pos.z.total_cmp(&0.5) {
                std::cmp::Ordering::Less => Side::Left,
                _ => Side::Right,
            },
        };

        match placement_rule {
            PlacementRule::Stairs => {
                block = block.set(PropName::Facing, direction.into_prop_value());
                block = block.set(PropName::Half, half.into_prop_value());
            }
            PlacementRule::Slabs => {
                if is_accumulating {
                    block = block.set(PropName::Type, PropValue::Double);
                } else {
                    block = block.set(PropName::Type, half.into_prop_value());
                }
            }
            //Where is the difference in these rules?
            PlacementRule::Buttons | PlacementRule::Lever => {
                //TODO: can be placed?

                let (face, facing) = match face {
                    Direction::Down => (PropValue::Ceiling, direction.into_prop_value()),
                    Direction::Up => (PropValue::Floor, direction.into_prop_value()),
                    Direction::North => (PropValue::Wall, PropValue::North),
                    Direction::South => (PropValue::Wall, PropValue::South),
                    Direction::West => (PropValue::Wall, PropValue::West),
                    Direction::East => (PropValue::Wall, PropValue::East),
                };

                block = block.set(PropName::Face, face);
                block = block.set(PropName::Facing, facing);
            }
            PlacementRule::Doors => {
                block = block.set(PropName::Facing, direction.into_prop_value());
                block = block.set(PropName::Half, PropValue::Lower);
                block = block.set(PropName::Hinge, side.into_prop_value());
                let block_below_pos = {
                    BlockPos {
                        y: new_block_pos.y - 1,
                        ..new_block_pos
                    }
                };
                let block_below = layer.block(block_below_pos).unwrap_or_default();
                if !can_be_placed_on_top(block, block_below.state) {
                    return;
                }

                let upper_block_pos = {
                    BlockPos {
                        y: new_block_pos.y + 1,
                        ..new_block_pos
                    }
                };
                block = block.set(PropName::Half, PropValue::Upper);
                if !layer
                    .block(upper_block_pos)
                    .unwrap_or_default()
                    .state
                    .is_replaceable()
                {
                    return;
                }
                layer.set_block(upper_block_pos, block);

                block = block.set(PropName::Half, PropValue::Lower);
            }
            PlacementRule::Basic | PlacementRule::Special => {}
            _ => {
                unimplemented!("PlacementRule not implemented yet.")
            }
        }

        layer.set_block(new_block_pos, block);
    }
}

//TODO: set_block should be the simpler version of this command and allow setting multi-blocks (i.e. doors) & have orientation etc.
