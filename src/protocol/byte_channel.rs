use std::sync::{Arc, Mutex};

use bytes::BytesMut;
use thiserror::Error;
use tokio::sync::Notify;

pub fn byte_channel(limit: usize) -> (ByteSender, ByteReceiver) {
    let shared = Arc::new(Shared {
        mtx: Mutex::new(Inner {
            bytes: BytesMut::new(),
            disconnected: false,
        }),
        notify: Notify::new(),
        limit,
    });

    let sender = ByteSender {
        shared: shared.clone(),
    };

    let receiver = ByteReceiver { shared };

    (sender, receiver)
}

pub struct ByteSender {
    shared: Arc<Shared>,
}

pub struct ByteReceiver {
    shared: Arc<Shared>,
}

struct Shared {
    mtx: Mutex<Inner>,
    notify: Notify,
    limit: usize,
}

struct Inner {
    bytes: BytesMut,
    disconnected: bool,
}

impl ByteSender {
    pub fn take_capacity(&mut self, additional: usize) -> BytesMut {
        let mut lck = self.shared.mtx.lock().unwrap();

        lck.bytes.reserve(additional);

        let len = lck.bytes.len();
        lck.bytes.split_off(len)
    }

    pub fn try_send(&mut self, mut bytes: BytesMut) -> Result<(), TrySendError> {
        let mut lck = self.shared.mtx.lock().unwrap();

        if lck.disconnected {
            return Err(TrySendError::Disconnected(bytes));
        }

        if bytes.is_empty() {
            return Ok(());
        }

        let available = self.shared.limit - lck.bytes.len();

        if bytes.len() > available {
            if available > 0 {
                lck.bytes.unsplit(bytes.split_to(available));
                self.shared.notify.notify_waiters();
            }

            return Err(TrySendError::Full(bytes));
        }

        lck.bytes.unsplit(bytes);
        self.shared.notify.notify_waiters();

        Ok(())
    }

    pub async fn send_async(&mut self, mut bytes: BytesMut) -> Result<(), SendError> {
        loop {
            let mut lck = self.shared.mtx.lock().unwrap();

            if lck.disconnected {
                return Err(SendError(bytes));
            }

            if bytes.is_empty() {
                return Ok(());
            }

            let available = self.shared.limit - lck.bytes.len();

            if bytes.len() > available {
                if available > 0 {
                    lck.bytes.unsplit(bytes.split_to(available));
                    self.shared.notify.notify_waiters();
                }
                drop(lck);

                self.shared.notify.notified().await;
            } else {
                lck.bytes.unsplit(bytes);
                self.shared.notify.notify_waiters();
                return Ok(());
            }
        }
    }

    pub fn is_disconnected(&self) -> bool {
        self.shared.mtx.lock().unwrap().disconnected
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum TrySendError {
    #[error("sender disconnected")]
    Disconnected(BytesMut),
    /// Contains any excess bytes not sent.
    #[error("channel full")]
    Full(BytesMut),
}

#[derive(Clone, PartialEq, Eq, Debug, Error)]
#[error("sender disconnected")]
pub struct SendError(pub BytesMut);

impl SendError {
    pub fn into_inner(self) -> BytesMut {
        self.0
    }
}

impl ByteReceiver {
    pub fn try_recv(&mut self) -> Result<BytesMut, TryRecvError> {
        let mut lck = self.shared.mtx.lock().unwrap();

        if !lck.bytes.is_empty() {
            self.shared.notify.notify_waiters();
            return Ok(lck.bytes.split());
        }

        if lck.disconnected {
            return Err(TryRecvError::Disconnected);
        }

        Err(TryRecvError::Empty)
    }

    pub async fn recv_async(&mut self) -> Result<BytesMut, RecvError> {
        loop {
            {
                let mut lck = self.shared.mtx.lock().unwrap();

                if !lck.bytes.is_empty() {
                    self.shared.notify.notify_waiters();
                    return Ok(lck.bytes.split());
                }

                if lck.disconnected {
                    return Err(RecvError::Disconnected);
                }
            }

            self.shared.notify.notified().await;
        }
    }

    pub fn is_disconnected(&self) -> bool {
        self.shared.mtx.lock().unwrap().disconnected
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error)]
pub enum TryRecvError {
    #[error("empty channel")]
    Empty,
    #[error("receiver disconnected")]
    Disconnected,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error)]
pub enum RecvError {
    #[error("receiver disconnected")]
    Disconnected,
}

impl Drop for ByteSender {
    fn drop(&mut self) {
        self.shared.mtx.lock().unwrap().disconnected = true;
    }
}

impl Drop for ByteReceiver {
    fn drop(&mut self) {
        self.shared.mtx.lock().unwrap().disconnected = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_channel_try() {
        let (mut sender, mut receiver) = byte_channel(4);

        assert_eq!(
            sender.try_send("hello".as_bytes().into()),
            Err(TrySendError::Full("o".as_bytes().into()))
        );

        assert_eq!(
            receiver.try_recv().unwrap(),
            BytesMut::from("hell".as_bytes())
        );
    }

    #[tokio::test]
    async fn byte_channel_async() {
        let (mut sender, mut receiver) = byte_channel(4);

        let t = tokio::spawn(async move {
            let bytes = receiver.recv_async().await.unwrap();
            assert_eq!(&bytes[..], b"hell");
            let bytes = receiver.recv_async().await.unwrap();
            assert_eq!(&bytes[..], b"o");

            assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
        });

        sender.send_async("hello".as_bytes().into()).await.unwrap();

        t.await.unwrap();

        assert!(sender.is_disconnected());
    }
}
