use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::protocol::packets::EncodePacket;

pub struct PacketReadBuf {
    /// `0..buf.len()` contains read bytes while `buf.len()..buf.capacity()` is
    /// space for more.
    buf: Vec<u8>,
    max_buf_size: usize,
}

impl PacketReadBuf {
    pub fn new(max_buf_size: usize) -> Self {
        assert!(max_buf_size <= isize::MAX as usize);

        Self {
            buf: Vec::with_capacity(max_buf_size.min(256)),
            max_buf_size,
        }
    }

    /// Continually reads bytes from a reader and appends it to the buffer. This
    /// stops when one of the following situations occur:
    ///
    /// - The reader reaches EOF (`Ok` is returned).
    /// - The maximum buffer capacity is reached (`Ok` is returned).
    /// - An IO error occurs (`Err` is returned).
    /// - The future is cancelled (nothing is returned).
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe.
    pub async fn fill_buf<R: AsyncRead>(&mut self, mut reader: R) -> io::Result<()> {
        loop {
            if self.buf.len() == self.buf.capacity() {
                let cap = self.buf.capacity();

                if cap >= self.max_buf_size {
                    // The buf is full.
                    return Ok(());
                }

                // Double our capacity without going over the configured maximum.
                //
                // The allocator might give us a bit more capacity than we request but that's
                // no big deal.
                self.buf
                    .reserve_exact(cap.min(self.max_buf_size.saturating_sub(cap)));
            }

            // Read up to the capacity of buf and no more.
            //
            // This would normally allocate more space in the buf if `buf.len() ==
            // buf.capacity()`, but we already handled that case above.
            let bytes_read = reader.read_buf(&mut self.buf).await?;

            if bytes_read == 0 {
                // EOF
                return Ok(());
            }
        }
    }
}
