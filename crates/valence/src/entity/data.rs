use bevy_ecs::prelude::*;
use tracing::warn;
use valence_protocol::Encode;

/// Cache for all the tracked data of an entity. Used for the
/// [`EntityTrackerUpdateS2c`][packet] packet.
///
/// [packet]: valence_protocol::packet::s2c::play::EntityTrackerUpdateS2c
#[derive(Component, Default)]
pub struct TrackedData {
    init_data: Vec<u8>,
    /// A map of tracked data indices to the offset in `init_data` where that
    /// particular value begins. The offsets are in ascending order. The indices
    /// are unordered.
    init_offsets: Vec<(u8, u32)>,
    update_data: Vec<u8>,
}

impl TrackedData {
    /// Returns initial tracked data for the entity, ready to be sent in the
    /// [`EntityTrackerUpdateS2c`][packet] packet. This is used when the entity
    /// enters the view of a client.
    ///
    /// [packet]: valence_protocol::packet::s2c::play::EntityTrackerUpdateS2c
    pub fn init_data(&self) -> Option<&[u8]> {
        if self.init_data.len() > 1 {
            Some(&self.init_data)
        } else {
            None
        }
    }

    /// Contains updated tracked data for the entity, ready to be sent in the
    /// [`EntityTrackerUpdateS2c`][packet] packet. This is used when tracked
    /// data is changed and the client is already in view of the entity.
    ///
    /// [packet]: valence_protocol::packet::s2c::play::EntityTrackerUpdateS2c
    pub fn update_data(&self) -> Option<&[u8]> {
        if self.update_data.len() > 1 {
            Some(&self.update_data)
        } else {
            None
        }
    }

    pub fn append_init_value(&mut self, index: u8, type_id: u8, value: &impl Encode) {
        debug_assert!(
            index != 0xff,
            "index of 0xff is reserved for the terminator"
        );

        self.init_data.pop(); // Remove terminator.

        // Append the new value to the end.
        debug_assert!(u32::try_from(self.init_data.len()).is_ok(), "too much data");
        let new_offset = self.init_data.len() as u32;

        self.init_data.extend_from_slice(&[index, type_id]);
        if let Err(e) = value.encode(&mut self.init_data) {
            warn!("failed to encode initial tracked data: {e:#}");
        }

        self.init_offsets.push((index, new_offset));

        self.init_data.push(0xff); // Add terminator.
    }

    pub fn remove_init_value(&mut self, index: u8) {
        // Is the index in the data?
        if let Some((offset_pos, (_, start))) = self
            .init_offsets
            .iter()
            .enumerate()
            .find(|(_, (idx, _))| *idx == index)
        {
            let start = *start as usize;

            let end = self
                .init_offsets
                .get(offset_pos + 1)
                .map(|(_, end)| *end as usize)
                .unwrap_or(self.init_data.len() - 1); // -1 to skip terminator.

            // Remove the range of bytes for the value.
            self.init_data.drain(start..end);
            self.init_offsets.remove(offset_pos);
        }
    }

    pub fn append_update_value(&mut self, index: u8, type_id: u8, value: &impl Encode) {
        debug_assert!(
            index != 0xff,
            "index of 0xff is reserved for the terminator"
        );

        self.update_data.pop(); // Remove terminator.

        self.update_data.extend_from_slice(&[index, type_id]);
        if let Err(e) = value.encode(&mut self.update_data) {
            warn!("failed to encode updated tracked data: {e:#}");
        }

        self.update_data.push(0xff); // Add terminator.
    }

    pub fn clear_update_values(&mut self) {
        self.update_data.clear();
    }
}
