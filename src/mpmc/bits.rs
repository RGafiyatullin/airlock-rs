use core::sync::atomic::AtomicUsize;

use crate::utils;

const POS_IS_CLOSED: u8 = 0;
const FLAGS_COUNT: u8 = 1;

type Usize = <AtomicUsize as crate::utils::AtomicValue>::Value;
const USIZE_BITS: u8 = Usize::BITS as u8;

const INDEX_BIT_COUNT: u8 = (USIZE_BITS - FLAGS_COUNT) / 4;
const START_HEAD_TAKEN: u8 = FLAGS_COUNT;
const START_HEAD_AVAIL: u8 = FLAGS_COUNT + INDEX_BIT_COUNT;
const START_TAIL_TAKEN: u8 = FLAGS_COUNT + INDEX_BIT_COUNT * 2;
const START_TAIL_AVAIL: u8 = FLAGS_COUNT + INDEX_BIT_COUNT * 3;

pub(super) fn is_closed(bits: Usize) -> bool {
    utils::bits::flag::<Usize, POS_IS_CLOSED>(bits) != 0
}
pub(super) fn set_closed(bits: Usize) -> Usize {
    bits | utils::bits::flag::<Usize, POS_IS_CLOSED>(utils::bits::ones())
}

pub(super) fn head_taken(bits: Usize) -> Usize {
    utils::bits::unpack::<Usize, START_HEAD_TAKEN, INDEX_BIT_COUNT>(bits)
}
pub(super) fn set_head_taken(bits: Usize, value: Usize) -> Usize {
    utils::bits::pack::<Usize, START_HEAD_TAKEN, INDEX_BIT_COUNT>(bits, value)
}

pub(super) fn head_avail(bits: Usize) -> Usize {
    utils::bits::unpack::<Usize, START_HEAD_AVAIL, INDEX_BIT_COUNT>(bits)
}
pub(super) fn set_head_avail(bits: Usize, value: Usize) -> Usize {
    utils::bits::pack::<Usize, START_HEAD_AVAIL, INDEX_BIT_COUNT>(bits, value)
}

pub(super) fn tail_taken(bits: Usize) -> Usize {
    utils::bits::unpack::<Usize, START_TAIL_TAKEN, INDEX_BIT_COUNT>(bits)
}
pub(super) fn set_tail_taken(bits: Usize, value: Usize) -> Usize {
    utils::bits::pack::<Usize, START_TAIL_TAKEN, INDEX_BIT_COUNT>(bits, value)
}

pub(super) fn tail_avail(bits: Usize) -> Usize {
    utils::bits::unpack::<Usize, START_TAIL_AVAIL, INDEX_BIT_COUNT>(bits)
}
pub(super) fn set_tail_avail(bits: Usize, value: Usize) -> Usize {
    utils::bits::pack::<Usize, START_TAIL_AVAIL, INDEX_BIT_COUNT>(bits, value)
}
