use core::sync::atomic::AtomicUsize;

use crate::utils;

type Usize = <AtomicUsize as crate::utils::AtomicValue>::Value;

const USIZE_BITS: u8 = Usize::BITS as u8;

const POS_IS_CLOSED: u8 = 0;
const POS_TX_IS_SET: u8 = 1;
const POS_RX_IS_SET: u8 = 2;

const FLAGS_COUNT: u8 = 3;

const INDEX_BIT_COUNT: u8 = (USIZE_BITS - FLAGS_COUNT) / 2;

const ONES: Usize = Usize::MAX;
const MASK_INDEX: Usize = !(ONES << INDEX_BIT_COUNT);

pub(super) fn max_len() -> Usize {
    MASK_INDEX
}

pub(super) mod is_closed {
    use super::*;

    pub fn is_set(bits: Usize) -> bool {
        utils::bits::flag::<Usize, POS_IS_CLOSED>(bits) != 0
    }

    pub fn set(bits: Usize) -> Usize {
        bits | utils::bits::flag::<Usize, POS_IS_CLOSED>(utils::bits::ones::<Usize>())
    }
}
pub(super) mod tx_is_set {
    use super::*;

    pub fn is_set(bits: Usize) -> bool {
        utils::bits::flag::<Usize, POS_TX_IS_SET>(bits) != 0
    }

    pub fn set(bits: Usize) -> Usize {
        bits | utils::bits::flag::<Usize, POS_TX_IS_SET>(utils::bits::ones::<Usize>())
    }
}
pub(super) mod rx_is_set {
    use super::*;

    pub fn is_set(bits: Usize) -> bool {
        utils::bits::flag::<Usize, POS_RX_IS_SET>(bits) != 0
    }

    pub fn set(bits: Usize) -> Usize {
        bits | utils::bits::flag::<Usize, POS_RX_IS_SET>(utils::bits::ones::<Usize>())
    }
}

pub(super) mod head {
    use super::*;

    const START: u8 = FLAGS_COUNT;
    const LEN: u8 = INDEX_BIT_COUNT;

    pub fn get(bits: Usize) -> Usize {
        utils::bits::unpack::<Usize, START, LEN>(bits)
    }
    pub fn set(bits: Usize, index: Usize) -> Usize {
        utils::bits::pack::<Usize, START, LEN>(bits, index)
    }
}
pub(super) mod tail {
    use super::*;

    const START: u8 = FLAGS_COUNT + INDEX_BIT_COUNT;
    const LEN: u8 = INDEX_BIT_COUNT;

    pub fn get(bits: Usize) -> Usize {
        utils::bits::unpack::<Usize, START, LEN>(bits)
    }
    pub fn set(bits: Usize, index: Usize) -> Usize {
        utils::bits::pack::<Usize, START, LEN>(bits, index)
    }
}

#[test]
fn test() {
    const N: Usize = 0xFF;

    for head in (0..N).chain((MASK_INDEX - N)..=MASK_INDEX) {
        for tail in (0..N).chain((MASK_INDEX - N)..=MASK_INDEX) {
            for closed in [true, false] {
                for tx_is_set in [true, false] {
                    for rx_is_set in [true, false] {
                        let bits = 0;

                        let bits = if closed { is_closed::set(bits) } else { bits };

                        let bits = if tx_is_set { tx_is_set::set(bits) } else { bits };

                        let bits = if rx_is_set { rx_is_set::set(bits) } else { bits };

                        let bits = head::set(bits, head);
                        let bits = tail::set(bits, tail);

                        assert_eq!(closed, is_closed::is_set(bits));
                        assert_eq!(tx_is_set, tx_is_set::is_set(bits));
                        assert_eq!(rx_is_set, rx_is_set::is_set(bits));
                        assert_eq!(head, head::get(bits));
                        assert_eq!(tail, tail::get(bits));
                    }
                }
            }
        }
    }
}
