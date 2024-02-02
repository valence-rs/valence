use valence_protocol::packets::play::CustomPayloadS2c;
use valence_protocol::{ident, Bounded, Encode, VarInt, WritePacket};

pub trait SetBrand {
    /// Sets the brand of the server.
    ///
    /// The Brand is displayed to the client in the F3 screen
    /// and is often used to display the server name.
    /// Any valid &str can be used.
    ///
    /// However, the legacy formatting codes are used,
    /// which means that while color and other formatting can technically be
    /// used, it needs to use the § character, which needs to be encoded as
    /// 0xC2, 0xA7.
    fn set_brand(&mut self, brand: &str);
}

impl<T: WritePacket> SetBrand for T {
    fn set_brand(&mut self, brand: &str) {
        let mut buf = vec![];
        let _ = VarInt(brand.len() as _).encode(&mut buf);
        buf.extend_from_slice(brand.as_bytes());
        self.write_packet(&CustomPayloadS2c {
            channel: ident!("minecraft:brand").into(),
            data: Bounded(buf.as_slice().into()),
        });
    }
}
