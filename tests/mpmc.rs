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

    let _tx_1 = Tx::new(&link);
    let _tx_2 = Tx::new(&link);

    let _rx_1 = Rx::new(&link);
    let _rx_2 = Rx::new(&link);
}

#[test]
#[should_panic]
fn t_02() {
    let tx_wakers = make_wakers::<1>();
    let rx_wakers = make_wakers::<1>();
    let buffer = make_buffer::<2>();
    let link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);

    let _tx_1 = Tx::new(&link);
    let _tx_2 = Tx::new(&link);
}

#[test]
#[should_panic]
fn t_03() {
    let tx_wakers = make_wakers::<1>();
    let rx_wakers = make_wakers::<1>();
    let buffer = make_buffer::<2>();
    let link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);

    let _rx_1 = Rx::new(&link);
    let _rx_2 = Rx::new(&link);
}

#[test]
fn t_04() {
    let counter = Counter::new();

    {
        let tx_wakers = make_wakers::<1>();
        let rx_wakers = make_wakers::<1>();
        let buffer = make_buffer::<2>();
        let link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);

        let mut tx_1 = Tx::new(&link);
        let mut rx_1 = Rx::new(&link);

        tx_1.send_nowait(counter.add(1)).expect("tx-1.send-nowait");
        assert!(tx_1.send_nowait(counter.add(1)).expect_err("tx-1.send-nowait").is_full());
    }
    assert_eq!(counter.count(), 0);
}

#[test]
fn t_05() {
    let counter = Counter::new();

    {
        let tx_wakers = make_wakers::<1>();
        let rx_wakers = make_wakers::<1>();
        let buffer = make_buffer::<2>();
        let link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);

        let mut tx_1 = Tx::new(&link);
        let mut rx_1 = Rx::new(&link);

        tx_1.send_nowait(counter.add(1)).expect("tx-1.send-nowait");
        assert_eq!(rx_1.recv_nowait().expect("rx-1.recv-nowait").unwrap(), 1);
        assert!(rx_1.recv_nowait().expect_err("rx-1.recv-nowait").is_empty());
    }
    assert_eq!(counter.count(), 0);
}

#[tokio::test]
async fn t_06() {
    let counter = Counter::new();

    {
        let tx_wakers = make_wakers::<2>();
        let rx_wakers = make_wakers::<2>();
        let buffer = make_buffer::<3>();
        let link = Link::<Value, _, _, _>::new(&buffer, &tx_wakers, &rx_wakers);

        let  mut tx_1 = Tx::new(&link);
        let mut tx_2 = Tx::new(&link);

        let mut rx_1 = Rx::new(&link);
        let mut rx_2 = Rx::new(&link);

        tx_1.send(counter.add(1)).await.expect("tx-1.send");
        tx_2.send(counter.add(2)).await.expect("tx-2.send");
        assert_eq!(rx_1.recv().await.expect("rx-1.recv").unwrap(), 1);
        assert_eq!(rx_2.recv().await.expect("rx-2.recv").unwrap(), 2);
    }
    assert_eq!(counter.count(), 0);
}

fn make_buffer<const SIZE: usize>() -> [Slot<Value>; SIZE] {
    core::array::from_fn(|_| Default::default())
}

fn make_wakers<const SIZE: usize>() -> [(AtomicBool, AtomicWaker); SIZE] {
    core::array::from_fn(|_| Default::default())
}
