use core::{fmt, borrow::Borrow};

use crate::mono::Link;

impl<T> fmt::Debug for crate::mono::Link<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L> fmt::Debug for crate::mono::Tx<T, L> where L: Borrow<Link<T>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}

impl<T, L> fmt::Debug for crate::mono::Rx<T, L> where L: Borrow<Link<T>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(core::any::type_name::<Self>()).finish()
    }
}
