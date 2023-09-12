use valence_protocol::encode::PacketWriter;
use valence_protocol::{CompressionThreshold, Encode, Packet, WritePacket};

#[derive(Debug)]
pub(crate) struct ActionBuf<T> {
    bytes: Vec<u8>,
    actions: Vec<(T, u32)>,
}

impl<T: PartialEq> ActionBuf<T> {
    pub const fn new() -> Self {
        Self {
            bytes: vec![],
            actions: vec![],
        }
    }

    pub fn push<U>(&mut self, action: T, f: impl FnOnce(&mut Vec<u8>) -> U) -> U {
        let before = self.bytes.len();
        let res = f(&mut self.bytes);
        let after = self.bytes.len();
        debug_assert!(before <= after);
        let len = (after - before) as u32;

        if let Some((prev_action, prev_len)) = self.actions.last_mut() {
            if action == *prev_action {
                *prev_len += len;
                return res;
            }
        }

        self.actions.push((action, len));
        res
    }

    pub fn packet_writer(
        &mut self,
        action: T,
        threshold: CompressionThreshold,
    ) -> impl WritePacket + '_
    where
        T: Clone,
    {
        struct Writer<'a, T> {
            buf: &'a mut ActionBuf<T>,
            action: T,
            threshold: CompressionThreshold,
        }

        impl<T: PartialEq + Clone> WritePacket for Writer<'_, T> {
            fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
            where
                P: Packet + Encode,
            {
                self.buf.push(self.action.clone(), |w| {
                    PacketWriter::new(&mut self.buf.bytes, self.threshold)
                        .write_packet_fallible(packet)
                })
            }

            fn write_packet_bytes(&mut self, bytes: &[u8]) {
                self.buf
                    .push(self.action.clone(), |w| w.extend_from_slice(bytes));
            }
        }

        Writer {
            buf: self,
            action,
            threshold,
        }
    }

    pub fn write_packet<P>(&mut self, action: T, threshold: CompressionThreshold, packet: &P)
    where
        P: Packet + Encode,
    {
        self.push(action, |w| {
            PacketWriter::new(&mut self.bytes, threshold).write_packet(packet)
        })
    }

    pub fn actions(&self) -> impl Iterator<Item = (&T, &[u8])> {
        let mut acc: usize = 0;

        self.actions.iter().map(move |(a, len)| {
            let slice = &self.bytes[acc..acc + *len as usize];
            acc += *len as usize;
            (a, slice)
        })
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
        self.actions.clear();
    }
}
