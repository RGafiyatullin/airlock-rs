use core::borrow::Borrow;
use core::fmt;
use core::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use crate::slot::Slot;

impl<T> fmt::Debug for crate::mono::Link<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L> fmt::Debug for crate::mono::Tx<T, L>
where
    L: Borrow<crate::mono::Link<T>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L> fmt::Debug for crate::mono::Rx<T, L>
where
    L: Borrow<crate::mono::Link<T>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, B> fmt::Debug for crate::buffered::Link<T, B>
where
    B: AsRef<[Slot<T>]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L, B> fmt::Debug for crate::buffered::Tx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<crate::buffered::Link<T, B>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L, B> fmt::Debug for crate::buffered::Rx<T, L, B>
where
    B: AsRef<[Slot<T>]>,
    L: Borrow<crate::buffered::Link<T, B>>,
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
