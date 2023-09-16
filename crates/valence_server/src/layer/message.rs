use bevy_ecs::prelude::*;
use valence_entity::{OldPosition, Position};
use valence_protocol::encode::PacketWriter;
use valence_protocol::{ChunkPos, CompressionThreshold, Encode, Packet, WritePacket};

use super::ChunkViewIndex;
use crate::Client;

#[derive(Component, Debug)]
pub struct LayerMessages {
    bytes: Vec<u8>,
    messages: Vec<Message>,
    threshold: CompressionThreshold,
}

pub(super) type Message = (MessageScope, MessageKind);

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum MessageScope {
    /// All clients viewing the layer will receive the message.
    #[default]
    All,
    /// Only the client identified by `only` will receive the message.
    Only {
        only: Entity,
    },
    /// All clients viewing the layer will receive the message, except the
    /// client identified by `except`.
    Except {
        except: Entity,
    },
    /// All clients in view of the chunk position `pos` will receive the
    /// message.
    ChunkView {
        pos: ChunkPos,
    },
    ChunkViewExcept {
        pos: ChunkPos,
        except: Entity,
    },
    /// All clients in view of `include` but _not_ in view of `exclude` will
    /// receive the message.
    TransitionChunkView {
        include: ChunkPos,
        exclude: ChunkPos,
    },
}

#[derive(Debug, Copy, Clone)]
pub(super) enum MessageKind {
    Packet { len: usize },
    EntityDespawn { entity: Entity },
}

impl LayerMessages {
    pub fn new(threshold: CompressionThreshold) -> Self {
        Self {
            bytes: vec![],
            messages: vec![],
            threshold,
        }
    }

    pub fn send_packet<P>(&mut self, scope: MessageScope, packet: &P)
    where
        P: Packet + Encode,
    {
        self.packet_writer(scope).write_packet(packet)
    }

    pub fn packet_writer(&mut self, scope: MessageScope) -> impl WritePacket + '_ {
        struct Writer<'a> {
            messages: &'a mut LayerMessages,
            scope: MessageScope,
        }

        impl WritePacket for Writer<'_> {
            fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
            where
                P: Packet + Encode,
            {
                let start = self.messages.bytes.len();
                let mut writer =
                    PacketWriter::new(&mut self.messages.bytes, self.messages.threshold);

                // Check if we're able to extend the last message instead of pushing a new one.
                if let Some((last_scope, last_kind)) = self.messages.messages.last_mut() {
                    if let MessageKind::Packet { len } = last_kind {
                        if *last_scope == self.scope {
                            writer.write_packet_fallible(packet)?;
                            let end = writer.buf.len();

                            *len += end - start;

                            return Ok(());
                        }
                    }
                }

                writer.write_packet_fallible(packet)?;
                let end = writer.buf.len();

                self.messages
                    .messages
                    .push((self.scope, MessageKind::Packet { len: end - start }));

                Ok(())
            }

            fn write_packet_bytes(&mut self, bytes: &[u8]) {
                self.messages.bytes.extend_from_slice(bytes);

                if let Some((last_scope, last_kind)) = self.messages.messages.last_mut() {
                    if let MessageKind::Packet { len } = last_kind {
                        if *last_scope == self.scope {
                            *len += bytes.len();

                            return;
                        }
                    }
                }

                self.messages
                    .messages
                    .push((self.scope, MessageKind::Packet { len: bytes.len() }));
            }
        }

        Writer {
            messages: self,
            scope,
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.bytes.clear();
    }

    pub fn threshold(&self) -> CompressionThreshold {
        self.threshold
    }

    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(super) fn messages(&self) -> impl Iterator<Item = Message> + '_ {
        self.messages.iter().copied()
    }
}

impl WritePacket for LayerMessages {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.packet_writer(MessageScope::All)
            .write_packet_fallible(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.packet_writer(MessageScope::All)
            .write_packet_bytes(bytes)
    }
}
