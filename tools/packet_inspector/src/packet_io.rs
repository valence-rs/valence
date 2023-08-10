use std::io;
use std::io::ErrorKind;

use anyhow::ensure;
use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use valence::protocol::decode::{PacketDecoder, PacketFrame};
use valence::protocol::encode::PacketEncoder;
use valence::protocol::{Encode, VarInt, MAX_PACKET_SIZE};
use valence::CompressionThreshold;

pub(crate) struct PacketIoReader {
    reader: tokio::io::ReadHalf<tokio::net::TcpStream>,
    dec: PacketDecoder,
    threshold: CompressionThreshold,
}

impl PacketIoReader {
    pub(crate) async fn recv_packet_raw(&mut self) -> anyhow::Result<PacketFrame> {
        loop {
            if let Some(frame) = self.dec.try_next_packet()? {
                // self.logger
                //     .log("Unknown".to_string(), self.direction.clone(), frame.clone());

                return Ok(frame);
            }

            self.dec.reserve(READ_BUF_SIZE);
            let mut buf = self.dec.take_capacity();

            if self.reader.read_buf(&mut buf).await? == 0 {
                return Err(io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            // This should always be an O(1) unsplit because we reserved space earlier and
            // the call to `read_buf` shouldn't have grown the allocation.
            self.dec.queue_bytes(buf);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_compression(&mut self, threshold: Option<u32>) {
        self.threshold = threshold;
        self.dec.set_compression(threshold);
    }
}

pub(crate) struct PacketIoWriter {
    writer: tokio::io::WriteHalf<tokio::net::TcpStream>,
    enc: PacketEncoder,
    threshold: CompressionThreshold,
}

impl PacketIoWriter {
    /*
      No  | Packet Length |  VarInt     | Length of (Data Length) + Compressed length of (Packet ID + Data)
      No  | Data Length   |  VarInt     | Length of uncompressed (Packet ID + Data) or 0
      Yes | Packet ID	  |  VarInt     | zlib compressed packet ID (see the sections below)
      Yes | Data          |  Byte Array | zlib compressed packet data (see the sections below)
    */
    pub(crate) async fn send_packet_raw(&mut self, frame: &PacketFrame) -> anyhow::Result<()> {
        let id_varint = VarInt(frame.id);
        let id_buf = varint_to_bytes(id_varint);

        let mut uncompressed_packet = BytesMut::new();
        uncompressed_packet.extend_from_slice(&id_buf);
        uncompressed_packet.extend_from_slice(&frame.body);
        let uncompressed_packet_length = uncompressed_packet.len();
        let uncompressed_packet_length_varint = VarInt(uncompressed_packet_length as i32);

        if let Some(threshold) = self.threshold {
            if uncompressed_packet_length > threshold as usize {
                use std::io::Read;

                use flate2::bufread::ZlibEncoder;
                use flate2::Compression;

                let mut z = ZlibEncoder::new(&uncompressed_packet[..], Compression::new(4));
                let mut compressed = Vec::new();

                let data_len_size = uncompressed_packet_length_varint.written_size();

                let packet_len = data_len_size + z.read_to_end(&mut compressed)?;

                ensure!(
                    packet_len <= MAX_PACKET_SIZE as usize,
                    "packet exceeds maximum length"
                );

                drop(z);

                self.enc
                    .append_bytes(&varint_to_bytes(VarInt(packet_len as i32)));

                self.enc
                    .append_bytes(&varint_to_bytes(uncompressed_packet_length_varint));

                self.enc.append_bytes(&compressed);

                let bytes = self.enc.take();

                self.writer.write_all(&bytes).await?;
                self.writer.flush().await?;

                // now we need to compress the packet.
            } else {
                // no need to compress, but we do need to inject a zero
                let empty = VarInt(0);

                let data_len_size = empty.written_size();
                let packet_len = data_len_size + uncompressed_packet_length;

                self.enc
                    .append_bytes(&varint_to_bytes(VarInt(packet_len as i32)));
                self.enc.append_bytes(&varint_to_bytes(empty));
                self.enc.append_bytes(&uncompressed_packet);
                let bytes = self.enc.take();
                self.writer.write_all(&bytes).await?;
                self.writer.flush().await?;
            }

            return Ok(());
        }

        let length = varint_to_bytes(VarInt(uncompressed_packet_length as i32));

        // the frame should be uncompressed at this point.
        self.enc.append_bytes(&length);
        self.enc.append_bytes(&uncompressed_packet);

        let bytes = self.enc.take();

        self.writer.write_all(&bytes).await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn set_compression(&mut self, threshold: Option<u32>) {
        self.threshold = threshold;
        self.enc.set_compression(threshold);
    }
}

pub(crate) struct PacketIo {
    stream: TcpStream,
    enc: PacketEncoder,
    dec: PacketDecoder,
    threshold: CompressionThreshold,
}

const READ_BUF_SIZE: usize = 1024;

impl PacketIo {
    pub(crate) fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            enc: PacketEncoder::new(),
            dec: PacketDecoder::new(),
            threshold: None,
        }
    }

    pub fn split(self) -> (PacketIoReader, PacketIoWriter) {
        let (reader, writer) = tokio::io::split(self.stream);

        (
            PacketIoReader {
                reader,
                dec: self.dec,
                threshold: self.threshold,
            },
            PacketIoWriter {
                writer,
                enc: self.enc,
                threshold: self.threshold,
            },
        )
    }

    #[allow(dead_code)]
    pub(crate) async fn set_compression(&mut self, threshold: Option<u32>) {
        self.threshold = threshold;
        self.enc.set_compression(threshold);
        self.dec.set_compression(threshold);
    }
}

pub fn varint_to_bytes(i: VarInt) -> BytesMut {
    let mut buf = BytesMut::new();
    let mut writer = (&mut buf).writer();
    i.encode(&mut writer).unwrap();

    buf
}
