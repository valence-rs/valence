use std::io::Write;
use std::marker::PhantomData;

use anyhow::anyhow;

use crate::{Encode, Result};

pub struct CachedEncode<T: ?Sized> {
    buf: Vec<u8>,
    res: Result<()>,
    _marker: PhantomData<fn(T) -> T>,
}

impl<T: Encode + ?Sized> CachedEncode<T> {
    pub fn new(t: &T) -> Self {
        let mut buf = Vec::new();
        let res = t.encode(&mut buf);

        Self {
            buf,
            res,
            _marker: PhantomData,
        }
    }

    pub fn set(&mut self, t: &T) {
        self.buf.clear();
        self.res = t.encode(&mut self.buf);
    }

    pub fn into_inner(self) -> Result<Vec<u8>> {
        self.res.map(|()| self.buf)
    }
}

impl<T: ?Sized> Encode for CachedEncode<T> {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        match &self.res {
            Ok(()) => Ok(w.write_all(&self.buf)?),
            Err(e) => Err(anyhow!("{e:#}")),
        }
    }

    fn encoded_len(&self) -> usize {
        self.buf.len()
    }
}
