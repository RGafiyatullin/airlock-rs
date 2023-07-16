use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

pub(crate) const ATOMIC_UPDATE_MAX_ITERATIONS: usize = 1024;

pub trait AtomicValue {
    type Value: Copy;

    fn load(&self, ordering: Ordering) -> Self::Value;
    fn compare_exchange(
        &self,
        old: Self::Value,
        new: Self::Value,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Value, Self::Value>;
}

pub(crate) fn compare_exchange_loop<A, F, E>(
    atomic_value: &A,
    max_attempts: usize,
    old_value: Option<A::Value>,
    mut map_value: F,
) -> Result<A::Value, Option<E>>
where
    A: AtomicValue,
    F: FnMut(A::Value) -> Result<A::Value, E>,
{
    let mut old_value = old_value.unwrap_or_else(|| atomic_value.load(Ordering::SeqCst));
    for _ in 0..max_attempts {
        let new_value = map_value(old_value).map_err(Some)?;
        match atomic_value.compare_exchange(
            old_value,
            new_value,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return Ok(new_value),
            Err(v) => old_value = v,
        }
    }
    Err(None)
}

macro_rules! impl_atomic_value {
    ($atomic: ty, $prim: ty) => {
        impl $crate::utils::AtomicValue for $atomic {
            type Value = $prim;

            #[inline(always)]
            fn load(&self, ordering: Ordering) -> Self::Value {
                Self::load(self, ordering)
            }

            #[inline(always)]
            fn compare_exchange(
                &self,
                old: Self::Value,
                new: Self::Value,
                success: Ordering,
                failure: Ordering,
            ) -> Result<Self::Value, Self::Value> {
                Self::compare_exchange(self, old, new, success, failure)
            }
        }
    };
}

impl_atomic_value!(AtomicU8, u8);
impl_atomic_value!(AtomicUsize, usize);

pub mod bits {
    use core::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};
    pub(crate) trait BitOps:
        From<u8>
        + Copy
        + Sized
        + Not<Output = Self>
        + BitAnd<Output = Self>
        + BitOr<Output = Self>
        + BitXor<Output = Self>
        + Shl<u8, Output = Self>
        + Shr<u8, Output = Self>
    {
    }
    impl<T> BitOps for T where
        T: From<u8>
            + Copy
            + Sized
            + Not<Output = Self>
            + BitAnd<Output = Self>
            + BitOr<Output = Self>
            + BitXor<Output = Self>
            + Shl<u8, Output = Self>
            + Shr<u8, Output = Self>
    {
    }

    pub(crate) fn flag<F: BitOps, const POS: u8>(flags: F) -> F {
        let one: F = 0b1u8.into();
        let mask = one << POS;
        flags & mask
    }

    pub(crate) fn unpack<F: BitOps, const START: u8, const LEN: u8>(packed: F) -> F {
        let ones: F = !F::from(0b0u8);
        let mask = (!(ones << LEN)) << START;
        let bits = packed & mask;
        bits >> START
    }

    pub(crate) fn pack<F: BitOps, const START: u8, const LEN: u8>(packed: F, value: F) -> F {
        let ones: F = !F::from(0b0u8);
        let mask = (!(ones << LEN)) << START;
        let bits = (value << START) & mask;
        (packed & !mask) | bits
    }
}
