use std::io::Write;

use valence_generated::attributes::{EntityAttribute, EntityAttributeOperation};
pub use valence_generated::item::ItemKind;
pub use valence_generated::sound::Sounds;
use valence_ident::Ident;
use valence_nbt::Compound;
use valence_text::Text;

use crate::{sound::SoundId, Decode, Encode, IDSet, VarInt};

/// A stack of items in an inventory.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ItemStack {
    pub item: ItemKind,
    pub count: i8,
    pub components: Vec<ItemComponent>,
}

type Ident<'a> = Ident<Cow<'a, str>>;

#[derive(Clone, PartialEq, Debug)]
pub enum ItemComponent<'a> {
    /// Customizable data that doesn't fit any specific component.
    CustomData {
        /// Always a Compound Tag.
        data: Compound,
    },
    /// Maximum stack size for the item.
    MaxStackSize {
        /// Ranges from 1 to 99.
        max_stack_size: VarInt,
    },
    /// The maximum damage the item can take before breaking.
    MaxDamage { max_damage: VarInt },
    /// The current damage of the item.
    Damage { damage: VarInt },
    /// Marks the item as unbreakable.
    Unbreakable {
        /// Whether the Unbreakable indicator should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// Item's custom name. Normally shown in italic, and changeable at an anvil.
    CustomName { name: Text },
    /// Item's model.
    ItemModel {
        /// The model identifier.
        model: String,
    },
    /// Override for the item's default name. Shown when the item has no custom name.
    ItemName { name: Text },
    /// Item's lore.
    Lore {
        /// The lore lines.
        lines: Vec<Text>,
    },
    /// Item's rarity. This affects the default color of the item's name.
    Rarity { rarity: Rarity },
    /// The enchantments of the item.
    Enchantments {
        /// The enchantments.
        enchantments: Vec<(VarInt, VarInt)>,
        /// Whether the enchantments should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// List of blocks this block can be placed on when in adventure mode.
    CanPlaceOn {
        /// The block predicates.
        block_predicates: Vec<BlockPredicate>,
        /// Whether the Unbreakable indicator should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// List of blocks this item can break when in adventure mode.
    CanBreak {
        /// The block predicates.
        block_predicates: Vec<BlockPredicate>,
        /// Whether the Unbreakable indicator should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// The attribute modifiers of the item.
    AttributeModifiers {
        /// The attributes.
        attributes: Vec<ItemAttribute>,
        /// Whether the modifiers should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// Value for the item predicate when using custom item models.
    CustomModelData { value: VarInt },
    /// Hides the special item's tooltip of crossbow ("Projectile:"), banner pattern layers, goat horn instrument and others.
    HideAdditionalTooltip,
    /// Hides the item's tooltip altogether.
    HideTooltip,
    /// Accumulated anvil usage cost.
    RepairCost { cost: VarInt },
    /// Marks the item as non-interactive on the creative inventory (the first 5 rows of items).
    CreativeSlotLock,
    /// Overrides the item glint resulted from enchantments.
    EnchantmentGlintOverride { has_glint: bool },
    /// Marks the projectile as intangible (cannot be picked-up).
    IntangibleProjectile,
    /// Makes the item restore players hunger when eaten.
    Food {
        /// Non-negative.
        nutrition: VarInt,
        /// How much saturation will be given after consuming the item.
        saturation_modifier: f32,
        /// Whether the item can always be eaten, even at full hunger.
        can_always_eat: bool,
    },
    /// Makes the item consumable.
    Consumable {
        /// How long it takes to consume the item.
        consume_seconds: f32,
        /// The animation type.
        animation: ConsumabeAnimation,
        /// The sound event.
        sound: SoundId,
        /// Whether the item has consume particles.
        has_consume_particles: bool,
        /// Number of elements in the following array.
        number_of_effects: VarInt,
        /// The effects.
        effects: Vec<u64>, // TODO
    },
    /// This specifies the item produced after using the current item.
    UseRemainder {
        /// The remainder item.
        remainder: ItemStack,
    },
    /// Cooldown to apply on use of the item.
    UseCooldown {
        /// The cooldown duration in seconds.
        seconds: f32,
        /// The cooldown group identifier.
        cooldown_group: Option<Ident<'a>>,
    },
    /// Marks this item as damage resistant.
    DamageResistant {
        /// Tag specifying damage types the item is immune to. Not prefixed by '#'.
        types: Ident<'a>,
    },
    /// Allows the item to be enchanted by an enchanting table.
    Enchantable {
        /// Opaque internal value controlling how expensive enchantments may be offered.
        value: VarInt,
    },
    /// Allows the item to be equipped by the player.
    Equippable {
        /// The slot type.
        slot: VarInt,
        /// The equip sound event.
        equip_sound: String,
        /// Whether the item has a model.
        has_model: bool,
        /// The model identifier. Only present if Has model is true.
        model: Option<String>,
        /// Whether the item has a camera overlay.
        has_camera_overlay: bool,
        /// The camera overlay identifier. Only present if Has camera overlay is true.
        camera_overlay: Option<String>,
        /// Whether the item has allowed entities.
        has_allowed_entities: bool,
        /// The allowed entities. Only present if Has allowed entities is true.
        allowed_entities: Option<Vec<String>>,
        /// Whether the item is dispensable.
        dispensable: bool,
        /// Whether the item is swappable.
        swappable: bool,
        /// Whether the item takes damage on hurt.
        damage_on_hurt: bool,
    },
    /// Items that can be combined with this item in an anvil to repair it.
    Repairable {
        /// The items.
        items: IDSet,
    },
    /// Makes the item function like elytra.
    Glider,
    /// Custom textures for the item tooltip.
    TooltipStyle {
        /// The style identifier.
        style: Ident<'a>,
    },
    /// Makes the item function like a totem of undying.
    DeathProtection {
        /// Number of elements in the following array.
        number_of_effects: VarInt,
        /// The effects.
        effects: Vec<u64>, // TODO
    },
    /// Alters the speed at which this item breaks certain blocks.
    Tool {
        /// The number of elements in the following array.
        number_of_rules: VarInt,
        /// The rules.
        rules: Vec<(
            Vec<VarInt>,
            bool,
            Option<f32>,
            bool,
            Option<bool>,
            f32,
            VarInt,
        )>,
    },
    /// The enchantments stored in this enchanted book.
    StoredEnchantments {
        /// Number of elements in the following array.
        number_of_enchantments: VarInt,
        /// The enchantments.
        enchantments: Vec<(VarInt, VarInt, bool)>,
    },
    /// Color of dyed leather armor.
    DyedColor {
        /// The RGB components of the color, encoded as an integer.
        color: VarInt,
        /// Whether the armor's color should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// Color of the markings on the map item model.
    MapColor {
        /// The RGB components of the color, encoded as an integer.
        color: VarInt,
    },
    /// The ID of the map.
    MapId { id: VarInt },
    /// Icons present on a map.
    MapDecorations {
        /// Always a Compound Tag.
        data: Compound,
    },
    /// Used internally by the client when expanding or locking a map. Display extra information on the item's tooltip when the component is present.
    MapPostProcessing {
        /// Type of post processing. Can be either:
        /// * 0 - Lock
        /// * 1 - Scale
        type_: VarInt,
    },
    /// Projectiles loaded into a charged crossbow.
    ChargedProjectiles {
        /// The number of elements in the following array.
        number_of_projectiles: VarInt,
        /// The projectiles.
        projectiles: Vec<ItemStack>,
    },
    /// Contents of a bundle.
    BundleContents {
        /// The number of elements in the following array.
        number_of_items: VarInt,
        /// The items.
        items: Vec<ItemStack>,
    },
    /// Visual and effects of a potion item.
    PotionContents {
        /// Whether this potion has an ID in the potion registry. If true, it has the default effects associated with the potion type.
        has_potion_id: bool,
        /// The ID of the potion type in the potion registry. Only present if Has Potion ID is true.
        potion_id: Option<VarInt>,
        /// Whether this potion has a custom color. If false, it uses the default color associated with the potion type.
        has_custom_color: bool,
        /// The RGB components of the color, encoded as an integer. Only present if Has Custom Color is true.
        custom_color: Option<VarInt>,
        /// The number of elements in the following array.
        number_of_custom_effects: VarInt,
        /// Any custom effects the potion might have.
        custom_effects: Vec<(
            VarInt,
            VarInt,
            bool,
            bool,
            bool,
            bool,
            Option<(VarInt, VarInt, bool, bool, bool, bool)>,
        )>,
        /// Custom name for the potion.
        custom_name: String,
    },
    /// Effects granted by a suspicious stew.
    SuspiciousStewEffects {
        /// Number of elements in the following array.
        number_of_effects: VarInt,
        /// The effects.
        effects: Vec<(VarInt, VarInt)>,
    },
    /// Content of a writable book.
    WritableBookContent {
        /// Number of elements in the following array.
        number_of_pages: VarInt,
        /// The pages.
        pages: Vec<(String, bool, Option<String>)>,
    },
    /// Content of a written and signed book.
    WrittenBookContent {
        /// The raw title of the book.
        raw_title: String,
        /// Whether the title has been filtered.
        has_filtered_title: bool,
        /// The title after going through chat filters. Only present if Has Filtered Title is true.
        filtered_title: Option<String>,
        /// The author of the book.
        author: String,
        /// The generation of the book.
        generation: VarInt,
        /// Number of elements in the following array.
        number_of_pages: VarInt,
        /// The pages.
        pages: Vec<(String, bool, Option<String>)>,
        /// Whether entity selectors have already been resolved.
        resolved: bool,
    },
    /// Armor's trim pattern and color.
    Trim {
        /// ID in the `minecraft:trim_material` registry, or an inline definition.
        trim_material: String,
        /// ID in the `minecraft:trim_pattern` registry, or an inline definition.
        trim_pattern: String,
        /// Whether the trim information should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// State of the debug stick.
    DebugStickState {
        /// States of previously interacted blocks. Always a Compound Tag.
        data: Compound,
    },
    /// Data for the entity to be created from this item.
    EntityData {
        /// Always a Compound Tag.
        data: Compound,
    },
    /// Data of the entity contained in this bucket.
    BucketEntityData {
        /// Always a Compound Tag.
        data: Compound,
    },
    /// Data of the block entity to be created from this item.
    BlockEntityData {
        /// Always a Compound Tag.
        data: Compound,
    },
    /// The sound played when using a goat horn.
    Instrument {
        /// ID in the `minecraft:instrument` registry, or an inline definition.
        instrument: String,
    },
    /// Amplifier for the effect of an ominous bottle.
    OminousBottleAmplifier {
        /// Between 0 and 4.
        amplifier: VarInt,
    },
    /// The song this item will play when inserted into a jukebox.
    JukeboxPlayable {
        /// Whether the jukebox song is specified directly, or just referenced by name.
        direct_mode: bool,
        /// The name of the jukebox song in its respective registry. Only present if Direct Mode is false.
        jukebox_song_name: Option<String>,
        /// ID in the `minecraft:jukebox_song` registry. Only present if Direct Mode is true.
        jukebox_song: Option<String>,
        /// Whether the song should be shown on the item's tooltip.
        show_in_tooltip: bool,
    },
    /// The recipes this knowledge book unlocks.
    Recipes {
        /// Always a Compound Tag.
        data: Compound,
    },
    /// The lodestone this compass points to.
    LodestoneTracker {
        /// Whether this lodestone points to a position, otherwise it spins randomly.
        has_global_position: bool,
        /// The dimension the compass points to. Only present if Has Global Position is true.
        dimension: Option<String>,
        /// The position the compass points to. Only present if Has Global Position is true.
        position: Option<(VarInt, VarInt, VarInt)>,
        /// Whether the component is removed when the associated lodestone is broken.
        tracked: bool,
    },
    /// Properties of a firework star.
    FireworkExplosion {
        /// See Firework Explosion.
        explosion: (VarInt, VarInt, Vec<VarInt>, VarInt, Vec<VarInt>, bool, bool),
    },
    /// Properties of a firework.
    Fireworks {
        /// The flight duration.
        flight_duration: VarInt,
        /// Number of elements in the following array.
        number_of_explosions: VarInt,
        /// The explosions.
        explosions: Vec<(VarInt, VarInt, Vec<VarInt>, VarInt, Vec<VarInt>, bool, bool)>,
    },
    /// Game Profile of a player's head.
    Profile {
        /// Whether the profile has a name.
        has_name: bool,
        /// The name of the profile. Only present if Has Name is true.
        name: Option<String>,
        /// Whether the profile has a unique ID.
        has_unique_id: bool,
        /// The unique ID of the profile. Only present if Has Unique ID is true.
        unique_id: Option<uuid::Uuid>,
        /// Number of elements in the following array.
        number_of_properties: VarInt,
        /// The properties.
        properties: Vec<(String, String, bool, Option<String>)>,
    },
    /// Sound played by a note block when this player's head is placed on top of it.
    NoteBlockSound {
        /// The sound.
        sound: String,
    },
    /// Patterns of a banner or banner applied to a shield.
    BannerPatterns {
        /// Number of elements in the following array.
        number_of_layers: VarInt,
        /// The layers.
        layers: Vec<(VarInt, Option<String>, Option<String>, VarInt)>,
    },
    /// Base color of the banner applied to a shield.
    BaseColor {
        /// The color.
        color: VarInt,
    },
    /// Decorations on the four sides of a pot.
    PotDecorations {
        /// The number of elements in the following array.
        number_of_decorations: VarInt,
        /// The decorations.
        decorations: Vec<VarInt>,
    },
    /// Items inside a container of any type.
    Container {
        /// The number of elements in the following array.
        number_of_items: VarInt,
        /// The items.
        items: Vec<ItemStack>,
    },
    /// State of a block.
    BlockState {
        /// Number of elements in the following array.
        number_of_properties: VarInt,
        /// The properties.
        properties: Vec<(String, String)>,
    },
    /// Bees inside a hive.
    Bees {
        /// Number of elements in the following array.
        number_of_bees: VarInt,
        /// The bees.
        bees: Vec<(Compound, VarInt, VarInt)>,
    },
    /// Name of the necessary key to open this container.
    Lock {
        /// Always a String Tag.
        key: String,
    },
    /// Loot table for an unopened container.
    ContainerLoot {
        /// Always a Compound Tag.
        data: Compound,
    },
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct BlockPredicate {
    pub blocks: Option<IDSet>,
    pub properties: Option<Vec<Property>>,
    pub nbt: Option<Compound>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Property {
    pub name: String,
    pub is_exact_match: bool,
    pub exact_value: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
}

impl Encode for Property {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.name.encode(&mut w)?;
        self.is_exact_match.encode(&mut w)?;
        if let Some(ref exact_value) = self.exact_value {
            exact_value.encode(&mut w)?;
        }
        if let Some(ref min_value) = self.min_value {
            min_value.encode(&mut w)?;
        }
        if let Some(ref max_value) = self.max_value {
            max_value.encode(&mut w)?;
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Property {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let name = String::decode(r)?;
        let is_exact_match = bool::decode(r)?;
        let exact_value = if is_exact_match {
            Some(String::decode(r)?)
        } else {
            None
        };
        let min_value = if !is_exact_match {
            Some(String::decode(r)?)
        } else {
            None
        };
        let max_value = if !is_exact_match {
            Some(String::decode(r)?)
        } else {
            None
        };
        Ok(Property {
            name,
            is_exact_match,
            exact_value,
            min_value,
            max_value,
        })
    }
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
struct ItemAttribute {
    pub effect: EntityAttribute,
    pub uuid: uuid::Uuid,
    pub name: String,
    pub value: f64,
    pub operation: EntityAttributeOperation,
    pub slot: AttributeSlot,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
enum AttributeSlot {
    Any = 0,
    MainHand = 1,
    OffHand = 2,
    Hand = 3,
    Feet = 4,
    Legs = 5,
    Chest = 6,
    Head = 7,
    Armor = 8,
    Body = 9,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
enum ConsumableAnimation {
    None,
    Eat,
    Drink,
    Block,
    Bow,
    Spear,
    Crossbow,
    Spyglass,
    TootHorn,
    Brush,
}

impl ItemComponent {
    fn id(self) -> u32 {
        match self {
            ItemComponent::CustomData { .. } => 0,
            ItemComponent::MaxStackSize { .. } => 1,
            ItemComponent::MaxDamage { .. } => 2,
            ItemComponent::Damage { .. } => 3,
            ItemComponent::Unbreakable { .. } => 4,
            ItemComponent::CustomName { .. } => 5,
            ItemComponent::ItemModel { .. } => 6,
            ItemComponent::ItemName { .. } => 7,
            ItemComponent::Lore { .. } => 8,
            ItemComponent::Rarity { .. } => 9,
            ItemComponent::Enchantments { .. } => 10,
            ItemComponent::CanPlaceOn { .. } => 11,
            ItemComponent::CanBreak { .. } => 12,
            ItemComponent::AttributeModifiers { .. } => 13,
            ItemComponent::CustomModelData { .. } => 14,
            ItemComponent::HideAdditionalTooltip => 15,
            ItemComponent::HideTooltip => 16,
            ItemComponent::RepairCost { .. } => 17,
            ItemComponent::CreativeSlotLock => 18,
            ItemComponent::EnchantmentGlintOverride { .. } => 19,
            ItemComponent::IntangibleProjectile => 20,
            ItemComponent::Food { .. } => 21,
            ItemComponent::Consumable { .. } => 22,
            ItemComponent::UseRemainder { .. } => 23,
            ItemComponent::UseCooldown { .. } => 24,
            ItemComponent::DamageResistant { .. } => 25,
            ItemComponent::Enchantable { .. } => 26,
            ItemComponent::Equippable { .. } => 27,
            ItemComponent::Repairable { .. } => 28,
            ItemComponent::Glider => 29,
            ItemComponent::TooltipStyle { .. } => 30,
            ItemComponent::DeathProtection { .. } => 31,
            ItemComponent::Tool { .. } => 32,
            ItemComponent::StoredEnchantments { .. } => 33,
            ItemComponent::DyedColor { .. } => 34,
            ItemComponent::MapColor { .. } => 35,
            ItemComponent::MapId { .. } => 36,
            ItemComponent::MapDecorations { .. } => 37,
            ItemComponent::MapPostProcessing { .. } => 38,
            ItemComponent::ChargedProjectiles { .. } => 39,
            ItemComponent::BundleContents { .. } => 40,
            ItemComponent::PotionContents { .. } => 41,
            ItemComponent::SuspiciousStewEffects { .. } => 42,
            ItemComponent::WritableBookContent { .. } => 43,
            ItemComponent::WrittenBookContent { .. } => 44,
            ItemComponent::Trim { .. } => 45,
            ItemComponent::DebugStickState { .. } => 46,
            ItemComponent::EntityData { .. } => 47,
            ItemComponent::BucketEntityData { .. } => 48,
            ItemComponent::BlockEntityData { .. } => 49,
            ItemComponent::Instrument { .. } => 50,
            ItemComponent::OminousBottleAmplifier { .. } => 51,
            ItemComponent::JukeboxPlayable { .. } => 52,
            ItemComponent::Recipes { .. } => 53,
            ItemComponent::LodestoneTracker { .. } => 54,
            ItemComponent::FireworkExplosion { .. } => 55,
            ItemComponent::Fireworks { .. } => 56,
            ItemComponent::Profile { .. } => 57,
            ItemComponent::NoteBlockSound { .. } => 58,
            ItemComponent::BannerPatterns { .. } => 59,
            ItemComponent::BaseColor { .. } => 60,
            ItemComponent::PotDecorations { .. } => 61,
            ItemComponent::Container { .. } => 62,
            ItemComponent::BlockState { .. } => 63,
            ItemComponent::Bees { .. } => 64,
            ItemComponent::Lock { .. } => 65,
            ItemComponent::ContainerLoot { .. } => 66,
        }
    }
}

impl ItemStack {
    pub const EMPTY: ItemStack = ItemStack {
        item: ItemKind::Air,
        count: 0,
        components: Vec::new(),
    };

    #[must_use]
    pub const fn new(item: ItemKind, count: i8, components: Vec<ItemComponent>) -> Self {
        Self {
            item,
            count,
            components,
        }
    }

    #[must_use]
    pub const fn with_count(mut self, count: i8) -> Self {
        self.count = count;
        self
    }

    #[must_use]
    pub const fn with_item(mut self, item: ItemKind) -> Self {
        self.item = item;
        self
    }

    #[must_use]
    pub fn with_components(mut self, components: Vec<ItemComponent>) -> Self {
        self.components = components;
        self
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self.item, ItemKind::Air) || self.count <= 0
    }
}

impl Encode for ItemStack {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        if self.is_empty() {
            0.encode(w)
        } else {
            self.count.encode(&mut w)?;
            self.item.encode(&mut w)?;
            self.components.encode(&mut w)
        }
    }
}

impl Decode<'_> for ItemStack {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(ItemStack::EMPTY);
        };

        let item = ItemKind::decode(r)?;
        let count = i8::decode(r)?;
        let components = Vec::<ItemComponent>::decode(r)?;

        let stack = ItemStack {
            item,
            count,
            components,
        };

        // Normalize empty item stacks.
        if stack.is_empty() {
            Ok(ItemStack::EMPTY)
        } else {
            Ok(stack)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_item_stack_is_empty() {
        let air_stack = ItemStack::new(ItemKind::Air, 10, Vec::new());
        let less_then_one_stack = ItemStack::new(ItemKind::Stone, 0, Vec::new());

        assert!(air_stack.is_empty());
        assert!(less_then_one_stack.is_empty());

        assert!(ItemStack::EMPTY.is_empty());

        let not_empty_stack = ItemStack::new(ItemKind::Stone, 10, Vec::new());

        assert!(!not_empty_stack.is_empty());
    }
}
