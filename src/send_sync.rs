mod slot {
    use crate::slot::Slot;

    unsafe impl<T: Send> Send for Slot<T> {}
    unsafe impl<T: Send> Sync for Slot<T> {}
}

mod spsc_direct {
    use crate::spsc::direct::*;
    use core::borrow::Borrow;

    unsafe impl<T: Send> Send for Link<T> {}
    unsafe impl<T: Send> Sync for Link<T> {}

    unsafe impl<T: Send, L: Send> Send for Tx<T, L> where L: Borrow<Link<T>> {}
    unsafe impl<T: Send, L: Sync> Sync for Tx<T, L> where L: Borrow<Link<T>> {}

    unsafe impl<T: Send, L: Send> Send for Rx<T, L> where L: Borrow<Link<T>> {}
    unsafe impl<T: Send, L: Sync> Sync for Rx<T, L> where L: Borrow<Link<T>> {}
}

mod spsc_buffered {
    use crate::slot::Slot;
    use crate::spsc::buffered::*;
    use core::borrow::Borrow;

    unsafe impl<T: Send, B: Send> Send for Link<T, B> where B: AsRef<[Slot<T>]> {}
    unsafe impl<T: Send, B: Sync> Sync for Link<T, B> where B: AsRef<[Slot<T>]> {}

    unsafe impl<T: Send, L: Send, B> Send for Tx<T, L, B>
    where
        L: Borrow<Link<T, B>>,
        B: AsRef<[Slot<T>]>,
    {
    }
    unsafe impl<T: Send, L: Sync, B> Sync for Tx<T, L, B>
    where
        L: Borrow<Link<T, B>>,
        B: AsRef<[Slot<T>]>,
    {
    }

    unsafe impl<T: Send, L: Send, B> Send for Rx<T, L, B>
    where
        L: Borrow<Link<T, B>>,
        B: AsRef<[Slot<T>]>,
    {
    }
    unsafe impl<T: Send, L: Sync, B> Sync for Rx<T, L, B>
    where
        L: Borrow<Link<T, B>>,
        B: AsRef<[Slot<T>]>,
    {
    }
}

mod mpmc {
    use core::borrow::Borrow;
    use core::sync::atomic::AtomicBool;

    use crate::atomic_waker::AtomicWaker;
    use crate::mpmc::*;
    use crate::slot::Slot;

    unsafe impl<T: Send, B: Send, TW: Send, RW: Send> Send for Link<T, B, TW, RW>
    where
        B: AsRef<[Slot<T>]>,
        TW: AsRef<[(AtomicBool, AtomicWaker)]>,
        RW: AsRef<[(AtomicBool, AtomicWaker)]>,
    {
    }
    unsafe impl<T: Send, B: Sync, TW: Sync, RW: Sync> Sync for Link<T, B, TW, RW>
    where
        B: AsRef<[Slot<T>]>,
        TW: AsRef<[(AtomicBool, AtomicWaker)]>,
        RW: AsRef<[(AtomicBool, AtomicWaker)]>,
    {
    }

    unsafe impl<T: Send, L: Send, B, TW, RW> Send for Tx<T, L, B, TW, RW>
    where
        L: Borrow<Link<T, B, TW, RW>>,
        B: AsRef<[Slot<T>]>,
        TW: AsRef<[(AtomicBool, AtomicWaker)]>,
        RW: AsRef<[(AtomicBool, AtomicWaker)]>,
    {
    }
    unsafe impl<T: Send, L: Sync, B, TW, RW> Sync for Tx<T, L, B, TW, RW>
    where
        L: Borrow<Link<T, B, TW, RW>>,
        B: AsRef<[Slot<T>]>,
        TW: AsRef<[(AtomicBool, AtomicWaker)]>,
        RW: AsRef<[(AtomicBool, AtomicWaker)]>,
    {
    }

    unsafe impl<T: Send, L: Send, B, TW, RW> Send for Rx<T, L, B, TW, RW>
    where
        L: Borrow<Link<T, B, TW, RW>>,
        B: AsRef<[Slot<T>]>,
        TW: AsRef<[(AtomicBool, AtomicWaker)]>,
        RW: AsRef<[(AtomicBool, AtomicWaker)]>,
    {
    }
    unsafe impl<T: Send, L: Sync, B, TW, RW> Sync for Rx<T, L, B, TW, RW>
    where
        L: Borrow<Link<T, B, TW, RW>>,
        B: AsRef<[Slot<T>]>,
        TW: AsRef<[(AtomicBool, AtomicWaker)]>,
        RW: AsRef<[(AtomicBool, AtomicWaker)]>,
    {
    }
}
