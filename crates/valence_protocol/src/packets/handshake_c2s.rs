use num_derive::{FromPrimitive, ToPrimitive};

use crate::{Handshaking, McRead, McWrite, Packet, PacketMeta, Serverbound};

#[derive(Copy, Clone, Debug)]
pub struct HandshakeC2s<'a> {
    pub protocol_version: i32,
    pub address: &'a str,
    pub port: i32,
    pub intended_state: ConnectionIntent,
}

impl Packet for HandshakeC2s<'_> {
    type Output<'a> = HandshakeC2s<'a>;

    fn read_body<'a>(r: &mut impl McRead<'a>) -> anyhow::Result<Self::Output<'a>> {
        Ok(HandshakeC2s {
            protocol_version: r.read_var_int()?,
            address: r.read_str_bounded(255)?,
            port: r.read_var_int()?,
            intended_state: r.read_enum::<ConnectionIntent>()?,
        })
    }

    fn write_body(&self, w: &mut impl McWrite) -> anyhow::Result<()> {
        w.write_var_int(self.protocol_version)?;
        w.write_str(self.address)?;
        w.write_var_int(self.port)?;
        w.write_enum(&self.intended_state)?;
        Ok(())
    }
    
}

impl PacketMeta<Handshaking, Serverbound> for HandshakeC2s<'_> {
    const ID: i32 = crate::id::handshaking::HANDSHAKE_C2S;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, FromPrimitive, ToPrimitive)]
pub enum ConnectionIntent {
    Status = 1,
    Login = 2,
}
