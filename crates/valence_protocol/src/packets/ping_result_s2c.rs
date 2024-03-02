use crate::{Clientbound, McRead, McWrite, Packet, PacketMeta, Status};

#[derive(Copy, Clone, Debug)]
pub struct PingResultS2c {
    pub start_time: i64,
}

impl Packet for PingResultS2c {
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

impl PacketMeta<Status, Clientbound> for PingResultS2c {
    const ID: i32 = crate::id::status::PING_RESULT_S2C;
}
