use std::sync::{Arc, atomic::AtomicBool};

use airlock::mpmc::*;
use airlock::slot::Slot;
use airlock::atomic_waker::AtomicWaker;

mod utils;
use futures::future;
use utils::{Counted, Counter};

type Value = Counted<usize>;

const BUFFER_SIZE: usize = 32;
const WAKERS_COUNT: usize = 32;

#[test]
fn t_00() {
    let tx_wakers = make_wakers::<WAKERS_COUNT>();
    let rx_wakers = make_wakers::<WAKERS_COUNT>();
    let buffer = make_buffer::<BUFFER_SIZE>();
    let _link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);
}

fn make_buffer<const SIZE: usize>() -> [Slot<Value>; SIZE] {
    core::array::from_fn(|_| Default::default())
}

fn make_wakers<const SIZE: usize>() -> [(AtomicBool, AtomicWaker); SIZE] {
    core::array::from_fn(|_| Default::default())
} 