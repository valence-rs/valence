use std::io::Read;

use flate2::read::{GzDecoder, ZlibDecoder};

use crate::error::{DataFormatError, Error};

#[derive(Debug, Copy, Clone)]
pub enum CompressionScheme {
    GZip = 1,
    Zlib = 2,
    Raw = 3,
}

impl CompressionScheme {
    pub(crate) fn from_raw(mode: u8) -> Result<Self, Error> {
        match mode {
            1 => Ok(Self::GZip),
            2 => Ok(Self::Zlib),
            3 => Ok(Self::Raw),
            scheme => Err(Error::DataFormatError(
                DataFormatError::UnknownCompressionScheme(scheme),
            )),
        }
    }

    pub(crate) fn read_to_vec<R: Read>(
        self,
        source: &mut R,
        length: usize,
    ) -> Result<Vec<u8>, std::io::Error> {
        let mut raw_data = vec![0u8; length];
        source.read_exact(&mut raw_data)?;
        match self {
            CompressionScheme::GZip => {
                let mut decoder = GzDecoder::new(std::io::Cursor::new(raw_data));
                let mut vec = Vec::<u8>::new();
                decoder.read_to_end(&mut vec)?;
                Ok(vec)
            }
            CompressionScheme::Zlib => {
                let mut decoder = ZlibDecoder::new(std::io::Cursor::new(raw_data));
                let mut vec = Vec::<u8>::new();
                decoder.read_to_end(&mut vec)?;
                Ok(vec)
            }
            CompressionScheme::Raw => Ok(raw_data),
        }
    }
}
