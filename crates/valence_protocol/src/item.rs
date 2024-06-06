use std::io::Write;

use uuid::Uuid;
pub use valence_generated::item::ItemKind;
use valence_nbt::{compound, Compound, List};

use crate::{Decode, Encode};

/// A stack of items in an inventory.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ItemStack {
    pub item: ItemKind,
    pub count: i8,
    pub nbt: Option<Compound>,
}

impl ItemStack {
    pub const EMPTY: ItemStack = ItemStack {
        item: ItemKind::Air,
        count: 0,
        nbt: None,
    };

    #[must_use]
    pub const fn new(item: ItemKind, count: i8, nbt: Option<Compound>) -> Self {
        Self { item, count, nbt }
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
    pub fn with_nbt(mut self, nbt: impl Into<Option<Compound>>) -> Self {
        self.nbt = nbt.into();
        self
    }

    /// This function takes the "Value" of the skin you want to apply to a
    /// PlayerHead. The "Value" is a Base64-encoded JSON object that is
    /// usually provided by websites. To learn more: <https://minecraft.wiki/w/Item_format#Player_Heads> 
    ///
    /// # Errors
    /// This function returns an error if the [ItemStack] you call it on isn't a PlayerHead
    ///
    /// # Examples
    /// ```
    /// // Value provided by https://minecraft-heads.com/
    /// let value = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvNzlmOWRkOGQ5MjQ0NTg0NWIwODM1MmZjMmY0OTRjYTE4OGJmNWMzNzFmM2JmOWQwMWJiNzRkOGVlNTk3YmM1YSJ9fX0=";
    /// let head = ItemStack::new(ItemKind::PlayerHead, 1, None).with_playerhead_texture_value(value).unwrap();
    /// ```
    ///
    /// Simple head command. More examples of Valence's command system: <https://github.com/valence-rs/valence/blob/main/examples/command.rs>
    /// ```
    /// #[derive(Command)]
    /// #[paths("head {username}")]
    /// struct HeadCommand {
    ///     username: String,
    /// }
    ///
    /// fn command_handler(
    ///     mut command_events: EventReader<CommandResultEvent<HeadCommand>>,
    ///     mut clients_query: Query<(&mut Client, &Username, &Properties, &mut Inventory)>,
    /// ) {
    ///     for event in command_events.read() {
    ///         let target_username = &event.result.username;
    ///
    ///         let target = if !target_username.is_empty() {
    ///             clients_query
    ///                 .iter()
    ///                 .find(|(_, username, _, _)| username.0 == *target_username)
    ///         } else {
    ///             Some(clients_query.get(event.executor).unwrap())
    ///         };
    ///
    ///         if let Some(target) = target {
    ///             let properties = target.2;
    ///             let textures = properties.textures().unwrap();
    ///
    ///             // Construct a PlayerHead using `with_playerhead_texture_value`
    ///             let head = ItemStack::new(ItemKind::PlayerHead, 1, None)
    ///                 .with_playerhead_texture_value(textures.value.clone())
    ///                 .unwrap();
    ///
    ///             let (_, _, _, mut inventory) = clients_query.get_mut(event.executor).unwrap();
    ///             inventory.set_slot(36, head);
    ///         } else {
    ///             let (mut client, _, _, _) = clients_query.get_mut(event.executor).unwrap();
    ///             client.send_chat_message(
    ///                 "No player with that username found on the server".color(Color::RED),
    ///             );
    ///         }
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn with_playerhead_texture_value(
        mut self,
        texture_value: impl Into<String>,
    ) -> Result<Self, ()> {
        if self.item != ItemKind::PlayerHead {
            return Err(());
        }

        let new_nbt = compound! {
            "SkullOwner" => compound! {
                "Id" => Uuid::default(),
                "Properties" => compound! {
                    "textures" => List::Compound(vec![
                        compound! {
                            "Value" => texture_value.into()
                        }
                    ])
                }
            }
        };

        if let Some(nbt) = &mut self.nbt {
            nbt.merge(new_nbt);
        } else {
            self.nbt = Some(new_nbt);
        }

        Ok(self)
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self.item, ItemKind::Air) || self.count <= 0
    }
}

impl Encode for ItemStack {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        if self.is_empty() {
            false.encode(w)
        } else {
            true.encode(&mut w)?;
            self.item.encode(&mut w)?;
            self.count.encode(&mut w)?;
            match &self.nbt {
                Some(n) => n.encode(w),
                None => 0u8.encode(w),
            }
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

        let nbt = if let [0, rest @ ..] = *r {
            *r = rest;
            None
        } else {
            Some(Compound::decode(r)?)
        };

        let stack = ItemStack { item, count, nbt };

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
        let air_stack = ItemStack::new(ItemKind::Air, 10, None);
        let less_then_one_stack = ItemStack::new(ItemKind::Stone, 0, None);

        assert!(air_stack.is_empty());
        assert!(less_then_one_stack.is_empty());

        assert!(ItemStack::EMPTY.is_empty());

        let not_empty_stack = ItemStack::new(ItemKind::Stone, 10, None);

        assert!(!not_empty_stack.is_empty());
    }
}
