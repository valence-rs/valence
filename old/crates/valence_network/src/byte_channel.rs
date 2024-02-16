//! A channel specifically for sending/receiving batches of bytes.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use bytes::BytesMut;
use thiserror::Error;
use tokio::sync::Notify;

pub(crate) fn byte_channel(limit: usize) -> (ByteSender, ByteReceiver) {
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

pub(crate) struct ByteSender {
    shared: Arc<Shared>,
}

pub(crate) struct ByteReceiver {
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
    pub(crate) fn take_capacity(&mut self, additional: usize) -> BytesMut {
        let mut lck = self.shared.mtx.lock().unwrap();

        lck.bytes.reserve(additional);

        let len = lck.bytes.len();
        lck.bytes.split_off(len)
    }

    pub(crate) fn try_send(&mut self, mut bytes: BytesMut) -> Result<(), TrySendError> {
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

    pub(crate) async fn send_async(&mut self, mut bytes: BytesMut) -> Result<(), SendError> {
        loop {
            {
                let mut lck = self.shared.mtx.lock().unwrap();

                if lck.disconnected {
                    return Err(SendError(bytes));
                }

                if bytes.is_empty() {
                    return Ok(());
                }

                let available = self.shared.limit - lck.bytes.len();

                if bytes.len() <= available {
                    lck.bytes.unsplit(bytes);
                    self.shared.notify.notify_waiters();
                    return Ok(());
                }

                if available > 0 {
                    lck.bytes.unsplit(bytes.split_to(available));
                    self.shared.notify.notify_waiters();
                }
            }

            self.shared.notify.notified().await;
        }
    }

    pub(crate) fn is_disconnected(&self) -> bool {
        self.shared.mtx.lock().unwrap().disconnected
    }

    pub(crate) fn limit(&self) -> usize {
        self.shared.limit
    }
}

/// Contains any excess bytes not sent.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub(crate) enum TrySendError {
    #[error("sender disconnected")]
    Disconnected(BytesMut),
    #[error("channel full (see `Config::outgoing_capacity`)")]
    Full(BytesMut),
}

#[derive(Clone, PartialEq, Eq, Debug, Error)]
#[error("sender disconnected")]
pub(crate) struct SendError(pub(crate) BytesMut);

impl SendError {
    pub(crate) fn into_inner(self) -> BytesMut {
        self.0
    }
}

impl ByteReceiver {
    pub(crate) fn try_recv(&mut self) -> Result<BytesMut, TryRecvError> {
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

    pub(crate) async fn recv_async(&mut self) -> Result<BytesMut, RecvError> {
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

    pub(crate) fn is_disconnected(&self) -> bool {
        self.shared.mtx.lock().unwrap().disconnected
    }

    pub(crate) fn limit(&self) -> usize {
        self.shared.limit
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error)]
pub(crate) enum TryRecvError {
    #[error("empty channel")]
    Empty,
    #[error("receiver disconnected")]
    Disconnected,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error)]
pub(crate) enum RecvError {
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
