use core::borrow::Borrow;
use core::convert::Infallible;
use core::future;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

use crate::atomic_waker::AtomicWaker;

use crate::error::{RecvError, RecvErrorNoWait, SendError, SendErrorNoWait};
use crate::slot::Slot;
use crate::utils;
use crate::utils::AtomicUpdate;

/// A medium through which [`Rx`] and [`Tx`] communicate.
pub struct Link<T, B>
where
    B: AsRef<[Slot<T>]>,
{
    /// 1bit — closed
    /// 1bit — tx is set
    /// 1bit — rx is set
    ///
    /// 14bit / 30bit — head
    /// 14bit / 30bit — tail
    ///
    /// 1bit — unused
    ///
    /// for 32bit usize max capacity — 16_384-1
    /// for 64bit usize max capacity — 1_073_741_824-1
    bits: AtomicUsize,

    tx_waker: AtomicWaker,
    rx_waker: AtomicWaker,

    _value: PhantomData<T>,

    buffer: B,
}

/// The sending side of the channel
pub struct Tx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<Link<T, B>>,
{
    link: L,
    _value: PhantomData<T>,
    _buffer: PhantomData<B>,
}

/// The receiving side of the channel
pub struct Rx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<Link<T, B>>,
{
    link: L,
    _value: PhantomData<T>,
    _buffer: PhantomData<B>,
}

impl<T, B> Link<T, B>
where
    B: AsRef<[Slot<T>]>,
{
    /// Creates a new ['Link`]
    pub fn new(buffer: B) -> Self {
        assert!(buffer.as_ref().len() < bits::max_len());

        Self {
            buffer,
            bits: Default::default(),
            tx_waker: Default::default(),
            rx_waker: Default::default(),
            _value: Default::default(),
        }
    }
}

impl<T, L, B> Tx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<Link<T, B>>,
{
    /// Creates a new [`Tx`]
    pub fn new(link: L) -> Self {
        link.borrow().set_tx();
        Self { link, _value: Default::default(), _buffer: Default::default() }
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

    /// Closes the channel.
    pub fn close(&mut self) {
        self.link.borrow().close(false, true)
    }
}

impl<T, L, B> Rx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<Link<T, B>>,
{
    /// Creates a new [`Rx`]
    pub fn new(link: L) -> Self {
        link.borrow().set_rx();
        Self { link, _value: Default::default(), _buffer: Default::default() }
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

    /// Closes the channel.
    pub fn close(&mut self) {
        self.link.borrow().close(false, true)
    }
}

impl<T, B> Link<T, B>
where
    B: AsRef<[Slot<T>]>,
{
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
        let bits = self.bits.load(Ordering::Relaxed);

        let buffer = self.buffer.as_ref();
        let buffer_len = buffer.len();

        let head = bits::head::get(bits);
        let tail = bits::tail::get(bits);
        let is_empty = head == tail;
        let is_closed = bits::is_closed::is_set(bits);

        match (is_empty, is_closed) {
            (true, true) => Err(RecvErrorNoWait::Closed),
            (true, false) => Err(RecvErrorNoWait::Empty),
            (false, _) => {
                let head_next = (head + 1) % buffer_len;
                let value = unsafe { buffer[head].as_maybe_uninit_mut().assume_init_read() };
                utils::compare_exchange_loop(
                    &self.bits,
                    self.max_iterations_for_atomic_update(),
                    Some(bits),
                    |old_bits| {
                        Ok::<_, Infallible>(AtomicUpdate::Set(bits::head::set(old_bits, head_next)))
                    },
                )
                .expect("failed to perform atomic update");
                self.tx_waker.wake();
                Ok(value)
            },
        }
    }

    fn send_nowait(&self, value: T) -> Result<(), SendErrorNoWait<T>> {
        let bits = self.bits.load(Ordering::Relaxed);

        let buffer = self.buffer.as_ref();
        let buffer_len = buffer.len();

        let head = bits::head::get(bits);
        let tail = bits::tail::get(bits);
        let tail_if_full = (head + buffer_len - 1) % buffer_len;
        let is_full = tail == tail_if_full;
        let is_closed = bits::is_closed::is_set(bits);

        match (is_closed, is_full) {
            (true, _) => Err(SendErrorNoWait::Closed(value)),
            (false, true) => Err(SendErrorNoWait::Full(value)),
            (false, false) => {
                let tail_next = (tail + 1) % buffer_len;
                unsafe { buffer[tail].as_maybe_uninit_mut() }.write(value);
                utils::compare_exchange_loop(
                    &self.bits,
                    self.max_iterations_for_atomic_update(),
                    Some(bits),
                    |old_bits| {
                        Ok::<_, Infallible>(AtomicUpdate::Set(bits::tail::set(old_bits, tail_next)))
                    },
                )
                .expect("failed to perform atomic update");

                self.rx_waker.wake();
                Ok(())
            },
        }
    }

    fn close(&self, notify_tx: bool, notify_rx: bool) {
        utils::compare_exchange_loop(
            &self.bits,
            self.max_iterations_for_atomic_update(),
            None,
            |old_flags| Ok::<_, Infallible>(AtomicUpdate::Set(bits::is_closed::set(old_flags))),
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
            &self.bits,
            self.max_iterations_for_atomic_update(),
            None,
            |old_bits| {
                if bits::tx_is_set::is_set(old_bits) {
                    Err("this link already has a Tx")
                } else {
                    Ok(AtomicUpdate::Set(bits::tx_is_set::set(old_bits)))
                }
            },
        ) {
            panic!("{}", err.unwrap_or("failed to perform atomic update"))
        }
    }
    fn set_rx(&self) {
        if let Err(err) = utils::compare_exchange_loop(
            &self.bits,
            self.max_iterations_for_atomic_update(),
            None,
            |old_bits| {
                if bits::rx_is_set::is_set(old_bits) {
                    Err("this link already has an Rx")
                } else {
                    Ok(AtomicUpdate::Set(bits::rx_is_set::set(old_bits)))
                }
            },
        ) {
            panic!("{}", err.unwrap_or("failed to perform atomic update"))
        }
    }

    fn max_iterations_for_atomic_update(&self) -> usize {
        utils::ATOMIC_UPDATE_MAX_ITERATIONS
    }
}

impl<T, B> Drop for Link<T, B>
where
    B: AsRef<[Slot<T>]>,
{
    fn drop(&mut self) {
        let bits = self.bits.load(Ordering::Relaxed);

        let is_closed = bits::is_closed::is_set(bits);
        let tx_is_set = bits::tx_is_set::is_set(bits);
        let rx_is_set = bits::rx_is_set::is_set(bits);

        if !is_closed && (tx_is_set || rx_is_set) {
            panic!("Dropping unclosed Link")
        }

        let mut head = bits::head::get(bits);
        let tail = bits::tail::get(bits);

        let slots = self.buffer.as_ref();

        while head != tail {
            unsafe {
                slots[head].as_maybe_uninit_mut().assume_init_drop();
            }

            head += 1;
        }
    }
}

impl<T, L, B> Drop for Tx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<Link<T, B>>,
{
    fn drop(&mut self) {
        self.link.borrow().close(/* notify_tx: */ false, /* notify_rx: */ true)
    }
}

impl<T, L, B> Drop for Rx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<Link<T, B>>,
{
    fn drop(&mut self) {
        self.link.borrow().close(/* notify_tx: */ true, /* notify_rx: */ false)
    }
}

mod bits;
