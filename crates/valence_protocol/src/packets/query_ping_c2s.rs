use crate::{McRead, McWrite, Packet, PacketMeta, Serverbound, Status};

#[derive(Copy, Clone, Debug)]
pub struct QueryPingC2s {
    pub start_time: i64,
}

impl Packet for QueryPingC2s {
    type Output<'a> = Self;

    fn read_body<'a>(r: &mut impl McRead<'a>) -> anyhow::Result<Self::Output<'a>> {
        Ok(Self {
            start_time: r.read_i64()?,
        })
    }

    fn write_body(&self, w: &mut impl McWrite) -> anyhow::Result<()> {
        w.write_i64(self.start_time)
    }
}

impl PacketMeta<Status, Serverbound> for QueryPingC2s {
    const ID: i32 = crate::id::status::QUERY_PING_C2S;
}
