use core::borrow::Borrow;
use core::convert::Infallible;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use futures::task::AtomicWaker;

use crate::error::{RecvErrorNoWait, SendErrorNoWait};
use crate::slot::Slot;
use crate::utils::{self, AtomicUpdate};

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
    fn send_nowait(&self, value: T) -> Result<(), SendErrorNoWait<T>> {
        let buffer = self.buffer.as_ref();
        let buffer_len = buffer.len();

        let bits = self.bits.load(Ordering::SeqCst);

        let head_avail = bits::head_avail(bits);
        let tail_taken = bits::tail_taken(bits); // FIXME: compare-exchange-loop to acquire this value
        let tail_if_full = (head_avail + buffer_len - 1) % buffer_len;
        let tail_avail_when_can_commit = (tail_taken + buffer_len - 1) % buffer_len;

        let is_full = tail_taken == tail_if_full;
        let is_closed = bits::is_closed(bits);

        match (is_closed, is_full) {
            (true, _) => Err(SendErrorNoWait::Closed(value)),
            (false, true) => Err(SendErrorNoWait::Full(value)),
            (false, false) => {
                unsafe { buffer[tail_taken].as_maybe_uninit_mut() }.write(value);
                utils::compare_exchange_loop(
                    &self.bits,
                    self.update_max_iterations(),
                    None,
                    |old_bits| {
                        if bits::tail_avail(old_bits) == tail_avail_when_can_commit {
                            Ok::<_, Infallible>(AtomicUpdate::Set(tail_taken))
                        } else {
                            Ok::<_, Infallible>(AtomicUpdate::Retry)
                        }
                    },
                )
                .expect("Failed to perform atomic update");

                Ok(())
            },
        }
    }

    fn recv_nowait(&self) -> Result<(), RecvErrorNoWait> {
        unimplemented!()
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
    fn update_max_iterations(&self) -> usize {
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
    }
}

mod bits {
    use core::sync::atomic::AtomicUsize;

    use crate::utils;

    const POS_IS_CLOSED: u8 = 0;
    const FLAGS_COUNT: u8 = 1;

    type Usize = <AtomicUsize as crate::utils::AtomicValue>::Value;
    const USIZE_BITS: u8 = Usize::BITS as u8;

    const INDEX_BIT_COUNT: u8 = (USIZE_BITS - FLAGS_COUNT) / 4;
    const START_HEAD_TAKEN: u8 = FLAGS_COUNT;
    const START_HEAD_AVAIL: u8 = FLAGS_COUNT + INDEX_BIT_COUNT * 1;
    const START_TAIL_TAKEN: u8 = FLAGS_COUNT + INDEX_BIT_COUNT * 2;
    const START_TAIL_AVAIL: u8 = FLAGS_COUNT + INDEX_BIT_COUNT * 3;

    pub(super) fn is_closed(bits: Usize) -> bool {
        utils::bits::flag::<Usize, POS_IS_CLOSED>(bits) != 0
    }
    pub(super) fn set_closed(bits: Usize) -> Usize {
        utils::bits::flag::<Usize, POS_IS_CLOSED>(utils::bits::ones())
    }

    pub(super) fn head_taken(bits: Usize) -> Usize {
        utils::bits::unpack::<Usize, START_HEAD_TAKEN, INDEX_BIT_COUNT>(bits)
    }
    pub(super) fn set_head_taken(bits: Usize, value: Usize) -> Usize {
        utils::bits::pack::<Usize, START_HEAD_TAKEN, INDEX_BIT_COUNT>(bits, value)
    }

    pub(super) fn head_avail(bits: Usize) -> Usize {
        utils::bits::unpack::<Usize, START_HEAD_AVAIL, INDEX_BIT_COUNT>(bits)
    }
    pub(super) fn set_head_avail(bits: Usize, value: Usize) -> Usize {
        utils::bits::pack::<Usize, START_HEAD_AVAIL, INDEX_BIT_COUNT>(bits, value)
    }

    pub(super) fn tail_taken(bits: Usize) -> Usize {
        utils::bits::unpack::<Usize, START_TAIL_TAKEN, INDEX_BIT_COUNT>(bits)
    }
    pub(super) fn set_tail_taken(bits: Usize, value: Usize) -> Usize {
        utils::bits::pack::<Usize, START_TAIL_TAKEN, INDEX_BIT_COUNT>(bits, value)
    }

    pub(super) fn tail_avail(bits: Usize) -> Usize {
        utils::bits::unpack::<Usize, START_TAIL_AVAIL, INDEX_BIT_COUNT>(bits)
    }
    pub(super) fn set_tail_avail(bits: Usize, value: Usize) -> Usize {
        utils::bits::pack::<Usize, START_TAIL_AVAIL, INDEX_BIT_COUNT>(bits, value)
    }
}
