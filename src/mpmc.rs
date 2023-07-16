use core::borrow::Borrow;
use core::convert::Infallible;
use core::future;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

use futures::task::AtomicWaker;

use crate::error::{RecvError, RecvErrorNoWait, SendError, SendErrorNoWait};
use crate::slot::Slot;
use crate::utils::{self, AtomicUpdate};

mod bits;

/// A medium through which [`Rx`] and [`Tx`] communicate.
pub struct Link<T, B, TW, RW>
where
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    _value: PhantomData<T>,

    buffer: B,

    refs: AtomicUsize,

    /// 1bit closed flag [0]
    /// four indexes (15/7bit):
    /// - head-taken     [ 1..=15 / 1..=7  ]
    /// - head-available [16..=30 / 8..=14 ]
    /// - tail-taken     [31..=45 / 15..=21]
    /// - tail-available [46..=61 / 22..=29]
    ///
    /// for 64bit usize max capacity — 32768-1
    /// for 32bit usize max capacity - 128-1
    bits: AtomicUsize,

    tx_wakers: TW,
    rx_wakers: RW,
}

/// The sending side of the channel
pub struct Tx<T, L, B, TW, RW>
where
    L: Borrow<Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    _value: PhantomData<T>,
    _buffer: PhantomData<B>,
    _tx_wakers: PhantomData<TW>,
    _rx_waker: PhantomData<RW>,

    link: L,
    idx: usize,
}

/// The receiving side of the channel
pub struct Rx<T, L, B, TW, RW>
where
    L: Borrow<Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    _value: PhantomData<T>,
    _buffer: PhantomData<B>,
    _tx_wakers: PhantomData<TW>,
    _rx_waker: PhantomData<RW>,

    link: L,
    idx: usize,
}

impl<T, L, B, TW, RW> Tx<T, L, B, TW, RW>
where
    L: Borrow<Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    /// Creates a new [`Tx`]
    pub fn new(link: L) -> Self {
        let idx = link.borrow().attach_tx();

        Self {
            _value: Default::default(),
            _buffer: Default::default(),
            _tx_wakers: Default::default(),
            _rx_waker: Default::default(),
            link,
            idx,
        }
    }

    /// Sends a value if the channel is not full.
    pub fn send_nowait(&mut self, value: T) -> Result<(), SendErrorNoWait<T>> {
        self.link.borrow().send_nowait(value)
    }

    /// Sends a value, waits if necessary.
    pub async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let mut value = Some(value);
        let link = self.link.borrow();
        future::poll_fn(|cx| link.poll_send(cx, self.idx, &mut value)).await
    }

    /// Closes the channel.
    pub fn close(&mut self) {
        self.link.borrow().close()
    }
}

impl<T, L, B, TW, RW> Rx<T, L, B, TW, RW>
where
    L: Borrow<Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    /// Creates a new [`Rx`]
    pub fn new(link: L) -> Self {
        let idx = link.borrow().attach_rx();

        Self {
            _value: Default::default(),
            _buffer: Default::default(),
            _tx_wakers: Default::default(),
            _rx_waker: Default::default(),
            link,
            idx,
        }
    }

    /// Receives a value if it is ready.
    pub fn recv_nowait(&mut self) -> Result<T, RecvErrorNoWait> {
        self.link.borrow().recv_nowait()
    }

    /// Receives a value, waits if necessary.
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        let link = self.link.borrow();
        future::poll_fn(|cx| link.poll_recv(cx, self.idx)).await
    }

    /// Closes the channel.
    pub fn close(&mut self) {
        self.link.borrow().close()
    }
}

impl<T, B, TW, RW> Link<T, B, TW, RW>
where
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    /// Creates a new [`Link`]
    pub fn new(buffer: B, tx_wakers: TW, rx_wakers: RW) -> Self {
        Self {
            _value: Default::default(),
            buffer,
            refs: Default::default(),
            bits: Default::default(),
            tx_wakers,
            rx_wakers,
        }
    }
}

impl<T, B, TW, RW> Link<T, B, TW, RW>
where
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn poll_recv(&self, cx: &mut Context, idx: usize) -> Poll<Result<T, RecvError>> {
        self.rx_wakers.as_ref()[idx].1.register(cx.waker());
        match self.recv_nowait() {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(RecvErrorNoWait::Closed) => Poll::Ready(Err(RecvError::closed())),
            Err(RecvErrorNoWait::Empty) => Poll::Pending,
        }
    }

    fn poll_send(
        &self,
        cx: &mut Context,
        idx: usize,
        value: &mut Option<T>,
    ) -> Poll<Result<(), SendError<T>>> {
        self.tx_wakers.as_ref()[idx].1.register(cx.waker());
        match self.send_nowait(value.take().expect("stolen value")) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(SendErrorNoWait::Closed(rejected)) => Poll::Ready(Err(SendError::closed(rejected))),
            Err(SendErrorNoWait::Full(rejected)) => {
                *value = Some(rejected);
                Poll::Pending
            },
        }
    }

    fn send_nowait(&self, value: T) -> Result<(), SendErrorNoWait<T>> {
        let buffer = self.buffer.as_ref();
        let buffer_len = buffer.len();

        let (tail_this, tail_next) = {
            let mut output = None;

            match utils::compare_exchange_loop(
                &self.bits,
                self.max_iterations_for_atomic_update(),
                None,
                |bits| {
                    let head_avail = bits::head_avail(bits);
                    let tail_taken = bits::tail_taken(bits);
                    let tail_taken_next = (tail_taken + 1) % buffer_len;
                    let tail_if_full = (head_avail + buffer_len - 1) % buffer_len;

                    let is_full = tail_taken == tail_if_full;
                    let is_closed = bits::is_closed(bits);

                    match (is_closed, is_full) {
                        (true, _) => Err(SendErrorNoWait::closed(())),
                        (false, true) => Err(SendErrorNoWait::full(())),
                        (false, false) => {
                            output = Some((tail_taken, tail_taken_next));
                            let new_bits = bits::set_tail_taken(bits, tail_taken_next);
                            Ok(AtomicUpdate::Set(new_bits))
                        },
                    }
                },
            ) {
                Ok(_) => output.unwrap(),
                Err(None) => panic!("Failed to perform atomic update"),
                Err(Some(e)) => return Err(e.map_value(value)),
            }
        };

        unsafe { buffer[tail_this].as_maybe_uninit_mut() }.write(value);

        utils::compare_exchange_loop(
            &self.bits,
            self.max_iterations_for_atomic_update(),
            None,
            |old_bits| {
                if bits::tail_avail(old_bits) == tail_this {
                    let new_bits = bits::set_tail_avail(old_bits, tail_next);
                    Ok::<_, Infallible>(AtomicUpdate::Set(new_bits))
                } else {
                    Ok::<_, Infallible>(AtomicUpdate::Retry)
                }
            },
        )
        .expect("Failed to perform atomic update");

        self.notify_rxs();

        Ok(())
    }

    fn recv_nowait(&self) -> Result<T, RecvErrorNoWait> {
        let buffer = self.buffer.as_ref();
        let buffer_len = buffer.len();

        let (head_this, head_next) = {
            let mut output = None;

            match utils::compare_exchange_loop(
                &self.bits,
                self.max_iterations_for_atomic_update(),
                None,
                |bits| {
                    let head_taken = bits::head_taken(bits);
                    let tail_avail = bits::tail_avail(bits);
                    let head_taken_next = (head_taken + 1) % buffer_len;
                    let is_empty = tail_avail == head_taken;
                    let is_closed = bits::is_closed(bits);

                    match (is_empty, is_closed) {
                        (true, true) => Err(RecvErrorNoWait::closed()),
                        (true, false) => Err(RecvErrorNoWait::empty()),
                        (false, _) => {
                            output = Some((head_taken, head_taken_next));
                            let new_bits = bits::set_head_taken(bits, head_taken_next);
                            Ok(AtomicUpdate::Set(new_bits))
                        },
                    }
                },
            ) {
                Ok(_) => output.unwrap(),
                Err(None) => panic!("Failed to perform atomic update"),
                Err(Some(e)) => return Err(e),
            }
        };

        let value = unsafe { buffer[head_this].as_maybe_uninit_mut().assume_init_read() };

        utils::compare_exchange_loop(
            &self.bits,
            self.max_iterations_for_atomic_update(),
            None,
            |old_bits| {
                if bits::head_avail(old_bits) == head_this {
                    let new_bits = bits::set_head_avail(old_bits, head_next);
                    Ok::<_, Infallible>(AtomicUpdate::Set(new_bits))
                } else {
                    Ok::<_, Infallible>(AtomicUpdate::Retry)
                }
            },
        )
        .expect("Failed to perform atomic update");

        self.notify_txs();

        Ok(value)
    }

    fn attach_tx(&self) -> usize {
        self.attach(self.tx_wakers.as_ref())
    }
    fn attach_rx(&self) -> usize {
        self.attach(self.rx_wakers.as_ref())
    }
    fn detach_tx(&self, idx: usize) {
        self.detach(self.tx_wakers.as_ref(), idx)
    }
    fn detach_rx(&self, idx: usize) {
        self.detach(self.rx_wakers.as_ref(), idx)
    }

    fn attach(&self, wakers: &[(AtomicBool, AtomicWaker)]) -> usize {
        for (idx, (taken, _waker)) in wakers.iter().enumerate() {
            if !taken.swap(true, Ordering::SeqCst) {
                self.ref_inc();
                return idx
            }
        }
        panic!("all wakers are taken")
    }

    fn detach(&self, wakers: &[(AtomicBool, AtomicWaker)], idx: usize) {
        let (taken, _) = &wakers[idx];
        if !taken.swap(false, Ordering::SeqCst) {
            panic!("attempt to detach from unoccupied waker")
        }
        self.ref_dec();
    }

    fn ref_inc(&self) {
        if self.refs.fetch_add(1, Ordering::SeqCst) == usize::MAX {
            panic!("ref-inc overflow")
        }
    }
    fn ref_dec(&self) {
        if self.refs.fetch_sub(1, Ordering::SeqCst) == 0 {
            panic!("ref-dec overflow")
        }
    }

    fn notify_rxs(&self) {
        Self::notify(self.rx_wakers.as_ref());
    }
    fn notify_txs(&self) {
        Self::notify(self.tx_wakers.as_ref());
    }
    fn notify(wakers: &[(AtomicBool, AtomicWaker)]) {
        for (_, waker) in wakers {
            waker.wake();
        }
    }

    fn close(&self) {
        utils::compare_exchange_loop(
            &self.bits,
            self.max_iterations_for_atomic_update(),
            None,
            |bits| Ok::<_, Infallible>(AtomicUpdate::Set(bits::set_closed(bits))),
        )
        .expect("failed to perform atomic update");

        self.notify_txs();
        self.notify_rxs();
    }

    fn max_iterations_for_atomic_update(&self) -> usize {
        utils::ATOMIC_UPDATE_MAX_ITERATIONS
    }
}

impl<T, L, B, TW, RW> Drop for Tx<T, L, B, TW, RW>
where
    L: Borrow<Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn drop(&mut self) {
        self.link.borrow().detach_tx(self.idx);
    }
}

impl<T, L, B, TW, RW> Drop for Rx<T, L, B, TW, RW>
where
    L: Borrow<Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn drop(&mut self) {
        self.link.borrow().detach_rx(self.idx);
    }
}

impl<T, B, TW, RW> Drop for Link<T, B, TW, RW>
where
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn drop(&mut self) {
        let refs = self.refs.load(Ordering::SeqCst);
        if refs != 0 {
            panic!("Dropping Link that is still referenced?")
        }

        let bits = self.bits.load(Ordering::SeqCst);
        let mut head = bits::head_avail(bits);
        let tail = bits::tail_avail(bits);
        assert_eq!(head, bits::head_taken(bits));
        assert_eq!(tail, bits::tail_taken(bits));

        let buffer = self.buffer.as_ref();
        let buffer_len = buffer.len();

        while head != tail {
            unsafe {
                buffer[head].as_maybe_uninit_mut().assume_init_drop();
            }

            head += 1;
            head %= buffer_len;
        }
    }
}
