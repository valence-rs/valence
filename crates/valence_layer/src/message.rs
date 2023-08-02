use core::fmt;
use std::convert::Infallible;
use std::ops::Range;

use valence_core::chunk_pos::{ChunkPos, ChunkView};

use crate::bvh::{ChunkBvh, GetChunkPos};

/// A message buffer of global messages (`G`) and local messages (`L`) meant for
/// consumption by clients. Local messages are those that have some spatial
/// component to them and implement the [`GetChunkPos`] trait. Local messages
/// are placed in a bounding volume hierarchy for fast queries via
/// [`Self::query_local`]. Global messages do not necessarily have a spatial
/// component and all globals will be visited when using [`Self::iter_global`].
///
/// Every message is associated with an arbitrary span of bytes. The meaning of
/// the bytes is whatever the message needs it to be.
///
/// At the end of the tick and before clients have access to the buffer, all
/// messages are sorted and then deduplicated by concatenating byte spans
/// together. This is done for a couple of reasons:
/// - Messages may rely on sorted message order for correctness, like in the
///   case of entity spawn & despawn messages. Sorting also makes deduplication
///   easy.
/// - Deduplication reduces the total number of messages that all clients must
///   examine. Consider the case of a message such as "send all clients in view
///   of this chunk position these packet bytes". If two of these messages have
///   the same chunk position, then they can just be combined together.
pub struct Messages<G, L> {
    global: Vec<(G, Range<u32>)>,
    local: Vec<(L, Range<u32>)>,
    bvh: ChunkBvh<MessagePair<L>>,
    staging: Vec<u8>,
    ready: Vec<u8>,
    is_ready: bool,
}

impl<G, L> Messages<G, L>
where
    G: Clone + Ord,
    L: Clone + Ord + GetChunkPos,
{
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Adds a global message to this message buffer.
    pub(crate) fn send_global<E>(
        &mut self,
        msg: G,
        f: impl FnOnce(&mut Vec<u8>) -> Result<(), E>,
    ) -> Result<(), E> {
        debug_assert!(!self.is_ready);

        let start = self.staging.len();
        f(&mut self.staging)?;
        let end = self.staging.len();

        if let Some((m, range)) = self.global.last_mut() {
            if msg == *m {
                // Extend the existing message.
                range.end = end as u32;
                return Ok(());
            }
        }

        self.global.push((msg, start as u32..end as u32));

        Ok(())
    }

    /// Adds a local message to this message buffer.
    pub(crate) fn send_local<E>(
        &mut self,
        msg: L,
        f: impl FnOnce(&mut Vec<u8>) -> Result<(), E>,
    ) -> Result<(), E> {
        debug_assert!(!self.is_ready);

        let start = self.staging.len();
        f(&mut self.staging)?;
        let end = self.staging.len();

        if let Some((m, range)) = self.local.last_mut() {
            if msg == *m {
                // Extend the existing message.
                range.end = end as u32;
                return Ok(());
            }
        }

        self.local.push((msg, start as u32..end as u32));

        Ok(())
    }

    /// Like [`Self::send_global`] but writing bytes cannot fail.
    pub(crate) fn send_global_infallible(&mut self, msg: G, f: impl FnOnce(&mut Vec<u8>)) {
        let _ = self.send_global::<Infallible>(msg, |b| {
            f(b);
            Ok(())
        });
    }

    /// Like [`Self::send_local`] but writing bytes cannot fail.
    pub(crate) fn send_local_infallible(&mut self, msg: L, f: impl FnOnce(&mut Vec<u8>)) {
        let _ = self.send_local::<Infallible>(msg, |b| {
            f(b);
            Ok(())
        });
    }

    /// Readies messages to be read by clients.
    pub(crate) fn ready(&mut self) {
        debug_assert!(!self.is_ready);
        self.is_ready = true;

        debug_assert!(self.ready.is_empty());

        self.ready.reserve_exact(self.staging.len());

        fn sort_and_merge<M: Clone + Ord>(
            msgs: &mut Vec<(M, Range<u32>)>,
            staging: &[u8],
            ready: &mut Vec<u8>,
        ) {
            // Sort must be stable.
            msgs.sort_by_key(|(msg, _)| msg.clone());

            // Make sure the first element is already copied to "ready".
            if let Some((_, range)) = msgs.first_mut() {
                let start = ready.len();
                ready.extend_from_slice(&staging[range.start as usize..range.end as usize]);
                let end = ready.len();

                *range = start as u32..end as u32;
            }

            msgs.dedup_by(|(right_msg, right_range), (left_msg, left_range)| {
                if *left_msg == *right_msg {
                    // Extend the left element with the right element. Then delete the right
                    // element.

                    let right_bytes =
                        &staging[right_range.start as usize..right_range.end as usize];

                    ready.extend_from_slice(right_bytes);

                    left_range.end += right_bytes.len() as u32;

                    true
                } else {
                    // Copy right element to "ready".

                    let right_bytes =
                        &staging[right_range.start as usize..right_range.end as usize];

                    let start = ready.len();
                    ready.extend_from_slice(right_bytes);
                    let end = ready.len();

                    *right_range = start as u32..end as u32;

                    false
                }
            });
        }

        sort_and_merge(&mut self.global, &self.staging, &mut self.ready);
        sort_and_merge(&mut self.local, &self.staging, &mut self.ready);

        self.bvh.build(
            self.local
                .iter()
                .cloned()
                .map(|(msg, range)| MessagePair { msg, range }),
        );
    }

    pub(crate) fn unready(&mut self) {
        assert!(self.is_ready);
        self.is_ready = false;

        self.local.clear();
        self.global.clear();
        self.staging.clear();
        self.ready.clear();
    }

    pub(crate) fn shrink_to_fit(&mut self) {
        self.global.shrink_to_fit();
        self.local.shrink_to_fit();
        self.bvh.shrink_to_fit();
        self.staging.shrink_to_fit();
        self.ready.shrink_to_fit();
    }

    /// All message bytes. Use this in conjunction with [`Self::iter_global`]
    /// and [`Self::query_local`].
    pub fn bytes(&self) -> &[u8] {
        debug_assert!(self.is_ready);

        &self.ready
    }

    /// Returns an iterator over all global messages and their span of bytes in
    /// [`Self::bytes`].
    pub fn iter_global(&self) -> impl Iterator<Item = (G, Range<usize>)> + '_ {
        debug_assert!(self.is_ready);

        self.global
            .iter()
            .map(|(m, r)| (m.clone(), r.start as usize..r.end as usize))
    }

    /// Takes a visitor function `f` and visits all local messages contained
    /// within the chunk view `view`. `f` is called with the local
    /// message and its span of bytes in [`Self::bytes`].
    pub fn query_local(&self, view: ChunkView, mut f: impl FnMut(L, Range<usize>)) {
        debug_assert!(self.is_ready);

        self.bvh.query(view, |pair| {
            f(
                pair.msg.clone(),
                pair.range.start as usize..pair.range.end as usize,
            )
        });
    }
}

impl<G, L> Default for Messages<G, L> {
    fn default() -> Self {
        Self {
            global: Default::default(),
            local: Default::default(),
            bvh: Default::default(),
            staging: Default::default(),
            ready: Default::default(),
            is_ready: Default::default(),
        }
    }
}

impl<G, L> fmt::Debug for Messages<G, L>
where
    G: fmt::Debug,
    L: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Messages")
            .field("global", &self.global)
            .field("local", &self.local)
            .field("is_ready", &self.is_ready)
            .finish()
    }
}

#[derive(Debug)]
struct MessagePair<M> {
    msg: M,
    range: Range<u32>,
}

impl<M: GetChunkPos> GetChunkPos for MessagePair<M> {
    fn chunk_pos(&self) -> ChunkPos {
        self.msg.chunk_pos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct DummyLocal;

    impl GetChunkPos for DummyLocal {
        fn chunk_pos(&self) -> ChunkPos {
            unimplemented!()
        }
    }

    #[test]
    fn send_global_message() {
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
        enum TestMsg {
            Foo,
            Bar,
        }

        let mut messages = Messages::<TestMsg, DummyLocal>::new();

        messages.send_global_infallible(TestMsg::Foo, |b| b.extend_from_slice(&[1, 2, 3]));
        messages.send_global_infallible(TestMsg::Bar, |b| b.extend_from_slice(&[4, 5, 6]));
        messages.send_global_infallible(TestMsg::Foo, |b| b.extend_from_slice(&[7, 8, 9]));

        messages.ready();

        let bytes = messages.bytes();

        for (msg, range) in messages.iter_global() {
            match msg {
                TestMsg::Foo => assert_eq!(&bytes[range.clone()], &[1, 2, 3, 7, 8, 9]),
                TestMsg::Bar => assert_eq!(&bytes[range.clone()], &[4, 5, 6]),
            }
        }

        messages.unready();
    }
}
