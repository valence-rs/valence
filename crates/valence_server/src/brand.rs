use valence_protocol::packets::play::CustomPayloadS2c;
use valence_protocol::{ident, Bounded, WritePacket};

pub trait SetBrand {
    /// Sets the brand of the server.
    ///
    /// The Brand is displayed to the client in the F3 screen
    /// and is often used to display the server name.
    /// Any valid &str can be used.
    fn set_brand(&mut self, brand: &str);
}

impl<T: WritePacket> SetBrand for T {
    fn set_brand(&mut self, brand: &str) {
        // The data is a &[u8], where the first byte is the length of the string.
        // It is also UTF-8 encoded, so any valid &str can be used.
        let vec_data = [brand.len() as u8]
            .iter()
            .chain(brand.as_bytes().iter())
            .copied()
            .collect::<Vec<_>>();
        let data = vec_data.as_slice();
        self.write_packet(&CustomPayloadS2c {
            channel: ident!("minecraft:brand").into(),
            data: Bounded(data.into()),
        });
    }
}
