use core::cell::UnsafeCell;
use core::fmt;
use core::mem::MaybeUninit;

pub struct Slot<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }
}

impl<T> fmt::Debug for Slot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T> Slot<T> {
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn as_maybe_uninit_mut(&self) -> &mut MaybeUninit<T> {
        let maybe_uninit_ptr = self.0.get();
        unsafe { maybe_uninit_ptr.as_mut() }.expect("UnsafeCell returned null_ptr?")
    }
}

unsafe impl<T> Send for Slot<T> where T: Send {}
unsafe impl<T> Sync for Slot<T> where T: Sync {}
