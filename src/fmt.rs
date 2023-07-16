use core::borrow::Borrow;
use core::fmt;
use core::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use crate::slot::Slot;

impl<T> fmt::Debug for crate::spsc::direct::Link<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L> fmt::Debug for crate::spsc::direct::Tx<T, L>
where
    L: Borrow<crate::spsc::direct::Link<T>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L> fmt::Debug for crate::spsc::direct::Rx<T, L>
where
    L: Borrow<crate::spsc::direct::Link<T>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, B> fmt::Debug for crate::spsc::buffered::Link<T, B>
where
    B: AsRef<[Slot<T>]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L, B> fmt::Debug for crate::spsc::buffered::Tx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<crate::spsc::buffered::Link<T, B>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L, B> fmt::Debug for crate::spsc::buffered::Rx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<crate::spsc::buffered::Link<T, B>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, B, TW, RW> fmt::Debug for crate::mpmc::Link<T, B, TW, RW>
where
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L, B, TW, RW> fmt::Debug for crate::mpmc::Tx<T, L, B, TW, RW>
where
    L: Borrow<crate::mpmc::Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L, B, TW, RW> fmt::Debug for crate::mpmc::Rx<T, L, B, TW, RW>
where
    L: Borrow<crate::mpmc::Link<T, B, TW, RW>>,
    B: AsRef<[Slot<T>]>,
    TW: AsRef<[(AtomicBool, AtomicWaker)]>,
    RW: AsRef<[(AtomicBool, AtomicWaker)]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}
