use std::io::Write;

use anyhow::{bail, ensure};
use byteorder::WriteBytesExt;
use derive_more::{From, Into};

use crate::{Decode, Encode, Packet, RawBytes, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityTrackerUpdateS2c<'a> {
    pub entity_id: VarInt,
    pub tracked_values: EntityTrackerBytes<'a>,
}

/// Marker byte to indicate the end of the data tracker bytes.
pub const MARKER_ID: u8 = 255;

/// Serialized entity data tracker bytes _without the terminating
/// [marker](MARKER_ID)_. The terminator is written by the [`Encode`] impl and
/// skipped by the [`Decode`] impl.
#[derive(Copy, Clone, PartialEq, Eq, Debug, From, Into)]
pub struct EntityTrackerBytes<'a>(pub &'a [u8]);

impl Encode for EntityTrackerBytes<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        ensure!(
            !self.0.contains(&MARKER_ID),
            "entity tracker bytes contains marker ID ({MARKER_ID}) when it should have been left \
             out"
        );

        RawBytes(self.0).encode(&mut w)?;
        w.write_u8(MARKER_ID)?;
        Ok(())
    }
}

impl<'a> Decode<'a> for EntityTrackerBytes<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        if let Some(idx) = r.iter().position(|&b| b == MARKER_ID) {
            let res = &r[..idx];
            *r = &r[idx + 1..];
            Ok(Self(res))
        } else {
            bail!("missing marker ID from input")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_bytes_decode() {
        let mut input = [1, 2, 3, 4, 5, MARKER_ID, 42, 42, 42].as_slice();

        assert_eq!(
            EntityTrackerBytes::decode(&mut input).unwrap(),
            EntityTrackerBytes(&[1, 2, 3, 4, 5])
        );
        assert_eq!(input, &[42, 42, 42]);
    }

    #[test]
    fn tracker_bytes_encode() {
        let mut out = vec![];

        EntityTrackerBytes(&[1, 2, 3, 4, 5])
            .encode(&mut out)
            .unwrap();

        assert_eq!(&out, &[1, 2, 3, 4, 5, MARKER_ID]);
    }
}
