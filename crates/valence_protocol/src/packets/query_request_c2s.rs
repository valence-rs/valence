use crate::{McRead, McWrite, Packet, PacketMeta, Serverbound, Status};

#[derive(Copy, Clone, Debug)]
pub struct QueryRequestC2s;

impl Packet for QueryRequestC2s {
    type Output<'a> = Self;

    fn read_body<'a>(_r: &mut impl McRead<'a>) -> anyhow::Result<Self::Output<'a>> {
        Ok(Self)
    }

    fn write_body(&self, _w: &mut impl McWrite) -> anyhow::Result<()> {
        Ok(())
    }
}

impl PacketMeta<Status, Serverbound> for QueryRequestC2s {
    const ID: i32 = crate::id::status::QUERY_REQUEST_C2S;
}
