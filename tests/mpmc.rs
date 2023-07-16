use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use airlock::atomic_waker::AtomicWaker;
use airlock::mpmc::*;
use airlock::slot::Slot;

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

#[test]
fn t_01() {
    let tx_wakers = make_wakers::<WAKERS_COUNT>();
    let rx_wakers = make_wakers::<WAKERS_COUNT>();
    let buffer = make_buffer::<BUFFER_SIZE>();
    let link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);

    let tx_1 = Tx::new(&link);
    let tx_2 = Tx::new(&link);

    let rx_1 = Rx::new(&link);
    let rx_2 = Rx::new(&link);
}

fn make_buffer<const SIZE: usize>() -> [Slot<Value>; SIZE] {
    core::array::from_fn(|_| Default::default())
}

fn make_wakers<const SIZE: usize>() -> [(AtomicBool, AtomicWaker); SIZE] {
    core::array::from_fn(|_| Default::default())
}
