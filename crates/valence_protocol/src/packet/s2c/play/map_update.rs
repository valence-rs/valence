use std::borrow::Cow;
use std::io::Write;

use crate::text::Text;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, EncodePacket, DecodePacket)]
#[packet_id = 0x25]
pub struct MapUpdateS2c<'a> {
    pub map_id: VarInt,
    pub scale: i8,
    pub locked: bool,
    pub icons: Option<Vec<Icon<'a>>>,
    pub data: Option<Data<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct Icon<'a> {
    pub icon_type: IconType,
    /// In map coordinates; -128 for furthest left, +127 for furthest right
    pub position: [i8; 2],
    /// 0 is a vertical icon and increments by 22.5°
    pub direction: i8,
    pub display_name: Option<Cow<'a, Text>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum IconType {
    WhiteArrow,
    GreenArrow,
    RedArrow,
    BlueArrow,
    WhiteCross,
    RedPointer,
    WhiteCircle,
    SmallWhiteCircle,
    Mansion,
    Temple,
    WhiteBanner,
    OrangeBanner,
    MagentaBanner,
    LightBlueBanner,
    YellowBanner,
    LimeBanner,
    PinkBanner,
    GrayBanner,
    LightGrayBanner,
    CyanBanner,
    PurpleBanner,
    BlueBanner,
    BrownBanner,
    GreenBanner,
    RedBanner,
    BlackBanner,
    TreasureMarker,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode)]
pub struct Data<'a> {
    pub columns: u8,
    pub rows: u8,
    pub position: [i8; 2],
    pub data: &'a [u8],
}

impl Encode for MapUpdateS2c<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.map_id.encode(&mut w)?;
        self.scale.encode(&mut w)?;
        self.locked.encode(&mut w)?;
        self.icons.encode(&mut w)?;

        match self.data {
            None => 0u8.encode(&mut w)?,
            Some(data) => data.encode(&mut w)?,
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for MapUpdateS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let map_id = VarInt::decode(r)?;
        let scale = i8::decode(r)?;
        let locked = bool::decode(r)?;
        let icons = <Option<Vec<Icon<'a>>>>::decode(r)?;
        let columns = u8::decode(r)?;

        let data = if columns > 0 {
            let rows = u8::decode(r)?;
            let position = <[i8; 2]>::decode(r)?;
            let data = <&'a [u8]>::decode(r)?;

            Some(Data {
                columns,
                rows,
                position,
                data,
            })
        } else {
            None
        };

        Ok(Self {
            map_id,
            scale,
            locked,
            icons,
            data,
        })
    }
}
