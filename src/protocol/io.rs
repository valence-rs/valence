use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use aes::Aes128;
use cfb8::Cfb8;
pub use incoming::{incoming, IncomingPacketReceiver, IncomingPacketSender};
pub use outgoing::{outgoing, OutgoingPacketReceiver, OutgoingPacketSender};
use tokio::time::timeout;

use crate::protocol::packets::{DecodePacket, EncodePacket};

mod incoming;
mod outgoing;

/// Manages a pair of [`IncomingPacketReceiver`] and [`OutgoingPacketSender`]
/// with timeouts on reads for convenience.
///
/// This is intended to be used in the initial packet exchange before the play
/// state begins.
pub struct PacketIoHandler {
    pub incoming: IncomingPacketReceiver,
    pub outgoing: OutgoingPacketSender,
    pub timeout: Duration,
}

impl PacketIoHandler {
    pub async fn send_packet<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: EncodePacket + ?Sized,
    {
        self.outgoing.append_packet(packet)?;
        self.outgoing.flush()
    }

    pub async fn recv_packet<P>(&mut self) -> anyhow::Result<P>
    where
        P: DecodePacket,
    {
        timeout(self.timeout, self.incoming.next_packet()).await?
    }

    pub fn set_compression(&mut self, threshold: Option<u32>) {
        self.incoming.set_compression(threshold.is_some());
        self.outgoing.set_compression(threshold);
    }

    pub fn enable_encryption(&mut self, key: &[u8; 16]) {
        self.incoming.enable_encryption(key);
        self.outgoing.enable_encryption(key);
    }
}

/// The AES block cipher with a 128 bit key, using the CFB-8 mode of
/// operation.
type Cipher = Cfb8<Aes128>;

struct PendingBytes {
    current: AtomicUsize,
    limit: usize,
}

impl PendingBytes {
    pub fn new(limit: usize) -> Self {
        Self {
            current: AtomicUsize::new(0),
            limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Context;

    use super::*;
    use crate::protocol::io::incoming::ReadLoop;
    use crate::protocol::io::outgoing::WriteLoop;
    use crate::protocol::packets::PacketName;
    use crate::protocol::{Decode, Encode};

    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestPacket {
        string: String,
        vec_of_u16: Vec<u16>,
        u64: u64,
    }

    impl PacketName for TestPacket {
        fn packet_name(&self) -> &'static str {
            "TestPacket"
        }
    }

    impl EncodePacket for TestPacket {
        fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
            self.string.encode(w)?;
            self.vec_of_u16.encode(w)?;
            self.u64.encode(w)
        }

        fn encoded_packet_len(&self) -> usize {
            self.string.encoded_len() + self.vec_of_u16.encoded_len() + self.u64.encoded_len()
        }
    }

    impl DecodePacket for TestPacket {
        fn decode_packet(r: &mut &[u8]) -> anyhow::Result<Self> {
            Ok(TestPacket {
                string: String::decode(r).context("decoding string field")?,
                vec_of_u16: Vec::decode(r).context("decoding vec of u16 field")?,
                u64: u64::decode(r).context("decoding u64 field")?,
            })
        }
    }

    impl TestPacket {
        fn new(s: impl Into<String>) -> Self {
            Self {
                string: s.into(),
                vec_of_u16: vec![0x1234, 0xabcd],
                u64: 0x1122334455667788,
            }
        }

        fn check(&self, s: impl AsRef<str>) {
            assert_eq!(&self.string, s.as_ref());
            assert_eq!(&self.vec_of_u16, &[0x1234, 0xabcd]);
            assert_eq!(self.u64, 0x1122334455667788);
        }
    }

    #[tokio::test]
    async fn packets_round_trip() {
        let mut buf = Vec::new();

        TestPacket::new("first").encode_packet(&mut buf).unwrap();
        debug_assert_eq!(
            TestPacket::decode_packet(&mut buf.as_slice()).unwrap(),
            TestPacket::new("first")
        );
        buf.clear();

        let (mut incoming_sender, mut incoming_receiver) = incoming(8192);
        let (mut outgoing_sender, mut outgoing_receiver) = outgoing(8192);

        outgoing_sender
            .append_packet(&TestPacket::new("first"))
            .unwrap();
        outgoing_sender.set_compression(Some(0));
        outgoing_sender
            .append_packet(&TestPacket::new("second"))
            .unwrap();
        outgoing_sender.flush().unwrap(); // Flush to avoid encrypting unflushed packets.
        outgoing_sender.enable_encryption(&CRYPT_KEY);
        outgoing_sender
            .append_packet(&TestPacket::new("third"))
            .unwrap();
        outgoing_sender
            .prepend_packet(&TestPacket::new("fourth"))
            .unwrap();
        outgoing_sender.flush().unwrap();

        assert_eq!(
            outgoing_receiver.write_loop(&mut buf, false).await.unwrap(),
            WriteLoop::Empty
        );

        assert!(!buf.is_empty());

        assert_eq!(
            incoming_sender.read_loop(buf.as_slice()).await.unwrap(),
            ReadLoop::Eof
        );

        incoming_receiver
            .next_packet::<TestPacket>()
            .await
            .unwrap()
            .check("first");
        incoming_receiver.set_compression(true);
        incoming_receiver
            .next_packet::<TestPacket>()
            .await
            .unwrap()
            .check("second");
        incoming_receiver.enable_encryption(&CRYPT_KEY);
        incoming_receiver
            .next_packet::<TestPacket>()
            .await
            .unwrap()
            .check("fourth");
        incoming_receiver
            .next_packet::<TestPacket>()
            .await
            .unwrap()
            .check("third");

        assert!(incoming_receiver
            .try_next_packet::<TestPacket>()
            .unwrap()
            .is_none());
    }
}
