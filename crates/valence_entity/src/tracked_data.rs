use bevy_ecs::prelude::*;
use tracing::warn;
use valence_core::protocol::Encode;

/// Cache for all the tracked data of an entity. Used for the
/// [`EntityTrackerUpdateS2c`][packet] packet.
///
/// [packet]: valence_packet::packets::play::EntityTrackerUpdateS2c
#[derive(Component, Default, Debug)]
pub struct TrackedData {
    init_data: Vec<u8>,
    /// A map of tracked data indices to the byte length of the entry in
    /// `init_data`.
    init_entries: Vec<(u8, u32)>,
    update_data: Vec<u8>,
}

impl TrackedData {
    /// Returns initial tracked data for the entity, ready to be sent in the
    /// [`EntityTrackerUpdateS2c`][packet] packet. This is used when the entity
    /// enters the view of a client.
    ///
    /// [packet]: valence_packet::packets::play::EntityTrackerUpdateS2c
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
    /// [packet]: valence_packet::packets::play::EntityTrackerUpdateS2c
    pub fn update_data(&self) -> Option<&[u8]> {
        if self.update_data.len() > 1 {
            Some(&self.update_data)
        } else {
            None
        }
    }

    pub fn insert_init_value(&mut self, index: u8, type_id: u8, value: impl Encode) {
        debug_assert!(
            index != 0xff,
            "index of 0xff is reserved for the terminator"
        );

        self.remove_init_value(index);

        self.init_data.pop(); // Remove terminator.

        // Append the new value to the end.
        let len_before = self.init_data.len();

        self.init_data.extend_from_slice(&[index, type_id]);
        if let Err(e) = value.encode(&mut self.init_data) {
            warn!("failed to encode initial tracked data: {e:#}");
        }

        let len = self.init_data.len() - len_before;

        self.init_entries.push((index, len as u32));

        self.init_data.push(0xff); // Add terminator.
    }

    pub fn remove_init_value(&mut self, index: u8) -> bool {
        let mut start = 0;

        for (pos, &(idx, len)) in self.init_entries.iter().enumerate() {
            if idx == index {
                let end = start + len as usize;

                self.init_data.drain(start..end);
                self.init_entries.remove(pos);

                return true;
            }

            start += len as usize;
        }

        false
    }

    pub fn append_update_value(&mut self, index: u8, type_id: u8, value: impl Encode) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_remove_init_tracked_data() {
        let mut td = TrackedData::default();

        td.insert_init_value(0, 3, "foo");
        td.insert_init_value(10, 6, "bar");
        td.insert_init_value(5, 9, "baz");

        assert!(td.remove_init_value(10));
        assert!(!td.remove_init_value(10));

        // Insertion overwrites value at index 0.
        td.insert_init_value(0, 64, "quux");

        assert!(td.remove_init_value(0));
        assert!(td.remove_init_value(5));

        assert!(td.init_data.as_slice().is_empty() || td.init_data.as_slice() == [0xff]);
        assert!(td.init_data().is_none());

        assert!(td.update_data.is_empty());
    }
}
