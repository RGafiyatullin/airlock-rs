use core::borrow::Borrow;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use futures::task::AtomicWaker;

use crate::slot::Slot;
use crate::utils;

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
    /// four indexes:
    /// - head-taken
    /// - head-available
    /// - tail-taken
    /// - tail-available
    ///
    /// for 64bit usize max capacity — 65536-1
    /// for 32bit usize max capacity - 256-1
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

mod bits {}
