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

pub(crate) enum AtomicUpdate<T> {
    Retry,
    Set(T),
}

pub(crate) fn compare_exchange_loop<A, F, E>(
    atomic_value: &A,
    max_attempts: usize,
    old_value: Option<A::Value>,
    mut map_value: F,
) -> Result<A::Value, Option<E>>
where
    A: AtomicValue,
    F: FnMut(A::Value) -> Result<AtomicUpdate<A::Value>, E>,
{
    let mut old_value = old_value.unwrap_or_else(|| atomic_value.load(Ordering::Relaxed));
    for _ in 0..max_attempts {
        let AtomicUpdate::Set(new_value) = map_value(old_value).map_err(Some)? else { continue };

        match atomic_value.compare_exchange(
            old_value,
            new_value,
            Ordering::Release,
            Ordering::Relaxed,
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

    pub(crate) fn zeroes<F: BitOps>() -> F {
        0b0u8.into()
    }
    pub(crate) fn ones<F: BitOps>() -> F {
        !zeroes::<F>()
    }

    pub(crate) fn flag<F: BitOps, const POS: u8>(flags: F) -> F {
        let one: F = 0b1u8.into();
        let mask = one << POS;
        flags & mask
    }

    pub(crate) fn unpack<F: BitOps, const START: u8, const LEN: u8>(packed: F) -> F {
        let ones: F = ones::<F>();
        let mask = !(ones << LEN);
        (packed >> START) & mask
    }

    pub(crate) fn pack<F: BitOps, const START: u8, const LEN: u8>(packed: F, value: F) -> F {
        let ones: F = ones::<F>();
        let mask = !(ones << LEN) << START;
        let bits = (value << START) & mask;
        (packed & !mask) | bits
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn tests() {
            assert_eq!(ones::<u8>(), 0b1111_1111);
            assert_eq!(flag::<u8, 0>(ones()), 0b0000_0001);
            assert_eq!(flag::<u8, 4>(ones()), 0b0001_0000);
            assert_eq!(flag::<u8, 7>(ones()), 0b1000_0000);

            assert_eq!(zeroes::<u8>(), 0b0000_0000);
            assert_eq!(flag::<u8, 0>(zeroes()), 0b0000_0000);
            assert_eq!(flag::<u8, 4>(zeroes()), 0b0000_0000);
            assert_eq!(flag::<u8, 7>(zeroes()), 0b0000_0000);

            assert_eq!(unpack::<u8, 1, 2>(ones()), 0b11);
            assert_eq!(pack::<u8, 1, 2>(0b0000_0000, 0b11), 0b0000_0110);
            assert_eq!(pack::<u8, 1, 2>(0b1111_1111, 0b11), 0b1111_1111);
            assert_eq!(pack::<u8, 1, 2>(0b0000_0000, 0b00), 0b0000_0000);
            assert_eq!(pack::<u8, 1, 2>(0b1111_1111, 0b00), 0b1111_1001);
        }
    }
}
