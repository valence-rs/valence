use bevy_ecs::entity::Entity;
use glam::DVec3;
use valence_core::{
    chunk_pos::ChunkPos,
    protocol::{
        encode::{PacketWriter, WritePacket},
        Encode, Packet,
    },
};

#[derive(Clone, Debug)]
pub struct MessageBuffer {
    messages: Vec<Message>,
    bytes: Vec<u8>,
    compression_threshold: Option<u32>,
}

impl MessageBuffer {
    pub fn new(compression_threshold: Option<u32>) -> Self {
        Self {
            messages: vec![],
            bytes: vec![],
            compression_threshold,
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn append_packet<P>(&mut self, cond: MessageCondition, pkt: &P)
    where
        P: Packet + Encode,
    {
        let threshold = self.compression_threshold;

        self.append(cond, |bytes| {
            PacketWriter::new(bytes, threshold).write_packet(pkt)
        })
    }

    pub fn append_packet_bytes(&mut self, cond: MessageCondition, bytes: &[u8]) {
        if !bytes.is_empty() {
            self.append(cond, |b| b.extend_from_slice(bytes));
        }
    }

    fn append(&mut self, cond: MessageCondition, append_data: impl FnOnce(&mut Vec<u8>)) {
        const LOOKBACK_BYTE_LIMIT: usize = 512;
        const LOOKBACK_MSG_LIMIT: usize = 64;

        // Look for a message with an identical condition to ours. If we find one, move it to the front and merge our message with it.

        let mut acc = 0;

        // Special case for the most recent message.
        if let Some(msg) = self.messages.last_mut() {
            if msg.cond == cond {
                let old_len = self.bytes.len();
                append_data(&mut self.bytes);
                let new_len = self.bytes.len();

                msg.len += new_len - old_len;

                return;
            }

            acc += msg.len;
        }

        for (i, msg) in self
            .messages
            .iter()
            .enumerate()
            .rev()
            .take(LOOKBACK_MSG_LIMIT)
            .skip(1)
        {
            acc += msg.len;

            if acc > LOOKBACK_BYTE_LIMIT {
                break;
            }

            if msg.cond == cond {
                let mut msg = self.messages.remove(i);

                let start = self.bytes.len() - acc;
                let range = start..start + msg.len;

                // Copy to the back and remove.
                self.bytes.extend_from_within(range.clone());
                self.bytes.drain(range);

                let old_len = self.bytes.len();
                append_data(&mut self.bytes);
                let new_len = self.bytes.len();

                msg.len += new_len - old_len;

                self.messages.push(msg);

                return;
            }
        }

        // Didn't find a compatible message, so append a new one to the end.

        let old_len = self.bytes.len();
        append_data(&mut self.bytes);
        let new_len = self.bytes.len();

        self.messages.push(Message {
            cond,
            len: new_len - old_len,
        });
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.bytes.clear();
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Message {
    pub cond: MessageCondition,
    /// Length of this message in bytes.
    pub len: usize,
}

impl Message {
    pub const fn new(cond: MessageCondition, len: usize) -> Self {
        Self { cond, len }
    }
}

/// A condition that must be met in order for a client to receive packet data.
#[derive(PartialEq, Copy, Clone, Default, Debug)]
pub enum MessageCondition {
    /// Data will be received unconditionally.
    #[default]
    All,
    /// All clients excluding this specific client.
    Except {
        client: Entity,
    },
    /// In view of this chunk position.
    View {
        pos: ChunkPos,
    },
    ViewExcept {
        pos: ChunkPos,
        except: Entity,
    },
    /// In view of `viewed` and not in view of `unviewed`.
    TransitionView {
        viewed: ChunkPos,
        unviewed: ChunkPos,
    },
    /// Client's position must be contained in this sphere.
    Sphere {
        center: DVec3,
        radius: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_buffer_append() {
        let cond1 = MessageCondition::All;
        let cond2 = MessageCondition::Except {
            client: Entity::PLACEHOLDER,
        };
        let cond3 = MessageCondition::Sphere {
            center: DVec3::ZERO,
            radius: 10.0,
        };

        let mut buf = MessageBuffer::new(None);

        let bytes = &[1, 2, 3, 4, 5];

        buf.append_packet_bytes(cond1, bytes);
        buf.append_packet_bytes(cond2, bytes);
        buf.append_packet_bytes(cond3, bytes);

        buf.append_packet_bytes(cond2, bytes);
        buf.append_packet_bytes(cond3, bytes);
        buf.append_packet_bytes(cond1, bytes);

        let msgs = buf.messages();

        assert_eq!(msgs[0], Message::new(cond2, bytes.len() * 2));
        assert_eq!(msgs[1], Message::new(cond3, bytes.len() * 2));
        assert_eq!(msgs[2], Message::new(cond1, bytes.len() * 2));
    }
}
