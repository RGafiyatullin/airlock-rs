use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use futures::task::AtomicWaker;

use crate::slot::Slot;

pub struct Link<T, B, TW, RW>
where
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    _value: PhantomData<T>,
    buffer: B,
    refs: AtomicUsize,
    bits: AtomicUsize,

    tx_wakers: TW,
    rx_wakers: RW,
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
