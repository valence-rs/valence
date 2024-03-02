use crate::{Clientbound, Packet, PacketMeta, Status};

#[derive(Copy, Clone, Debug)]
pub struct QueryResponseS2c<'a> {
    pub json: &'a str
}

impl Packet for QueryResponseS2c<'_> {
    type Output<'a> = QueryResponseS2c<'a>;

    fn read_body<'a>(r: &mut impl crate::McRead<'a>) -> anyhow::Result<Self::Output<'a>> {
        Ok(QueryResponseS2c {
            json: r.read_str()?,
        })
    }

    fn write_body(&self, w: &mut impl crate::McWrite) -> anyhow::Result<()> {
        w.write_str(self.json)
    }
}

impl PacketMeta<Status, Clientbound> for QueryResponseS2c<'_> {
    const ID: i32 = crate::id::status::QUERY_RESPONSE_S2C;
}
