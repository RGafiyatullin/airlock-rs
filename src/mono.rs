use core::borrow::Borrow;
use core::convert::Infallible;
use core::future;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU8, Ordering};
use core::task::{Context, Poll};

use futures::task::AtomicWaker;

use crate::error::{RecvError, RecvErrorNoWait, SendError, SendErrorNoWait};
use crate::slot::Slot;
use crate::utils;

const FLAG_IS_CLOSED: u8 = 0b0001;
const FLAG_IS_FULL: u8 = 0b0010;
const FLAG_TX_IS_SET: u8 = 0b0100;
const FLAG_RX_IS_SET: u8 = 0b1000;
const ATOMIC_UPDATE_MAX_ITERATIONS: usize = 1024;

/// A medium through which [`Rx`] and [`Tx`] communicate.
pub struct Link<T> {
    flags: AtomicU8,
    rx_waker: AtomicWaker,
    tx_waker: AtomicWaker,
    slot: Slot<T>,
}

/// The receiving side of the channel
pub struct Rx<T, L>
where
    L: Borrow<Link<T>>,
{
    link: L,
    _value: PhantomData<T>,
}

/// The sending side of the channel
pub struct Tx<T, L>
where
    L: Borrow<Link<T>>,
{
    link: L,
    _value: PhantomData<T>,
}

impl<T, L> Rx<T, L>
where
    L: Borrow<Link<T>>,
{
    /// Creates a new [`Rx`].
    pub fn new(link: L) -> Self {
        link.borrow().set_rx();
        Self { link, _value: Default::default() }
    }

    /// Receives a value if it is ready.
    pub fn recv_nowait(&mut self) -> Result<T, RecvErrorNoWait> {
        self.link.borrow().recv_nowait()
    }

    /// Receives a value, waits if necessary.
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        let link = self.link.borrow();
        future::poll_fn(|cx| link.poll_recv(cx)).await
    }

    pub fn close(&mut self) {
        self.link.borrow().close(true, false)
    }
}

impl<T, L> Tx<T, L>
where
    L: Borrow<Link<T>>,
{
    /// Creates a new [`Tx`].
    pub fn new(link: L) -> Self {
        link.borrow().set_tx();
        Self { link, _value: Default::default() }
    }

    /// Sends a value if the channel is not full.
    pub fn send_nowait(&mut self, value: T) -> Result<(), SendErrorNoWait<T>> {
        self.link.borrow().send_nowait(value)
    }

    /// Sends a value, waits if necessary.
    pub async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let mut value = Some(value);
        let link = self.link.borrow();
        future::poll_fn(|cx| link.poll_send(cx, &mut value)).await
    }

    pub fn close(&mut self) {
        self.link.borrow().close(false, true)
    }
}

impl<T> Link<T> {
    /// Creates a new ['Link`]
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> Link<T> {
    fn poll_recv(&self, cx: &mut Context) -> Poll<Result<T, RecvError>> {
        self.rx_waker.register(cx.waker());
        match self.recv_nowait() {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(RecvErrorNoWait::Closed) => Poll::Ready(Err(RecvError::closed())),
            Err(RecvErrorNoWait::Empty) => Poll::Pending,
        }
    }

    fn poll_send(&self, cx: &mut Context, value: &mut Option<T>) -> Poll<Result<(), SendError<T>>> {
        self.tx_waker.register(cx.waker());
        match self.send_nowait(value.take().expect("stolen value")) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(SendErrorNoWait::Closed(rejected)) => Poll::Ready(Err(SendError::closed(rejected))),
            Err(SendErrorNoWait::Full(rejected)) => {
                *value = Some(rejected);
                Poll::Pending
            },
        }
    }

    fn recv_nowait(&self) -> Result<T, RecvErrorNoWait> {
        let flags = self.flags.load(Ordering::SeqCst);

        let is_closed = flags & FLAG_IS_CLOSED != 0;
        let is_empty = flags & FLAG_IS_FULL == 0;

        match (is_closed, is_empty) {
            (false, true) => Err(RecvErrorNoWait::empty()),
            (true, true) => Err(RecvErrorNoWait::closed()),

            (_, false) => {
                let value = unsafe { self.slot.as_maybe_uninit_mut().assume_init_read() };

                utils::compare_exchange_loop(
                    &self.flags,
                    self.update_max_iterations(),
                    Some(flags),
                    |old_flags| Ok::<_, Infallible>(old_flags & !FLAG_IS_FULL),
                )
                .expect("failed to perform atomic update");

                self.tx_waker.wake();

                Ok(value)
            },
        }
    }

    fn send_nowait(&self, value: T) -> Result<(), SendErrorNoWait<T>> {
        let flags = self.flags.load(Ordering::SeqCst);

        let is_closed = flags & FLAG_IS_CLOSED != 0;
        let is_full = flags & FLAG_IS_FULL != 0;

        match (is_closed, is_full) {
            (true, _) => Err(SendErrorNoWait::closed(value)),
            (false, true) => Err(SendErrorNoWait::full(value)),
            (false, false) => {
                unsafe { self.slot.as_maybe_uninit_mut() }.write(value);

                utils::compare_exchange_loop(
                    &self.flags,
                    self.update_max_iterations(),
                    Some(flags),
                    |old_flags| Ok::<_, Infallible>(old_flags | FLAG_IS_FULL),
                )
                .expect("failed to perform atomic update");

                self.rx_waker.wake();

                Ok(())
            },
        }
    }

    fn close(&self, notify_tx: bool, notify_rx: bool) {
        utils::compare_exchange_loop(
            &self.flags,
            self.update_max_iterations(),
            None,
            |old_flags| Ok::<_, Infallible>(old_flags | FLAG_IS_CLOSED),
        )
        .expect("failed to perform atomic update");

        if notify_tx {
            self.tx_waker.wake();
        }
        if notify_rx {
            self.rx_waker.wake();
        }
    }

    fn set_tx(&self) {
        if let Err(err) = utils::compare_exchange_loop(
            &self.flags,
            self.update_max_iterations(),
            None,
            |old_flags| {
                if old_flags & FLAG_TX_IS_SET != 0 {
                    Err("this link already has a Tx")
                } else {
                    Ok(old_flags | FLAG_TX_IS_SET)
                }
            },
        ) {
            panic!("{}", err.unwrap_or("failed to perform atomic update"))
        }
    }
    fn set_rx(&self) {
        if let Err(err) = utils::compare_exchange_loop(
            &self.flags,
            self.update_max_iterations(),
            None,
            |old_flags| {
                if old_flags & FLAG_RX_IS_SET != 0 {
                    Err("this link already has a Rx")
                } else {
                    Ok(old_flags | FLAG_RX_IS_SET)
                }
            },
        ) {
            panic!("{}", err.unwrap_or("failed to perform atomic update"))
        }
    }

    fn update_max_iterations(&self) -> usize {
        ATOMIC_UPDATE_MAX_ITERATIONS
    }
}

impl<T> Default for Link<T> {
    fn default() -> Self {
        Self {
            flags: Default::default(),
            rx_waker: Default::default(),
            tx_waker: Default::default(),
            slot: Default::default(),
        }
    }
}

impl<T> Drop for Link<T> {
    fn drop(&mut self) {
        if self.flags.load(Ordering::SeqCst) & FLAG_IS_FULL != 0 {
            unsafe {
                self.slot.as_maybe_uninit_mut().assume_init_drop();
            }
        }
    }
}

impl<T, L> Drop for Rx<T, L>
where
    L: Borrow<Link<T>>,
{
    fn drop(&mut self) {
        self.link.borrow().close(/* notify_tx: */ true, /* notify_rx: */ false);
    }
}

impl<T, L> Drop for Tx<T, L>
where
    L: Borrow<Link<T>>,
{
    fn drop(&mut self) {
        self.link.borrow().close(/* notify_tx: */ false, /* notify_rx: */ true);
    }
}
