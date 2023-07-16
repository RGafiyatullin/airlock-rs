use std::sync::Arc;

use airlock::spsc::direct::*;

mod utils;
use futures::future;
use utils::{Counted, Counter};

type Value = Counted<usize>;

#[test]
fn t_00() {
    let _link = Link::<Value>::new();
}

#[test]
fn t_01() {
    let counter = Counter::new();

    {
        let link = Link::<Value>::new();
        let mut rx = Rx::new(&link);
        let mut tx = Tx::new(&link);

        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_empty());

        tx.send_nowait(counter.add(1)).expect("tx.send-nowait");
        assert!(tx.send_nowait(counter.add(2)).expect_err("tx.send-nowait").is_full());

        assert_eq!(rx.recv_nowait().expect("rx.recv-nowait").unwrap(), 1);
        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_empty());

        tx.send_nowait(counter.add(3)).expect("tx.send-nowait");
        assert!(tx.send_nowait(counter.add(4)).expect_err("tx.send-nowait").is_full());

        assert_eq!(rx.recv_nowait().expect("rx.recv-nowait").unwrap(), 3);
    }

    assert_eq!(counter.count(), 0);
}

#[test]
fn t_02() {
    let counter = Counter::new();

    {
        let link = Link::<Value>::new();
        let mut tx = Tx::new(&link);

        tx.send_nowait(counter.add(1)).expect("tx.send-nowait");
        assert!(tx.send_nowait(counter.add(2)).expect_err("tx.send-nowait").is_full());
    }

    assert_eq!(counter.count(), 0);
}

#[test]
fn t_03() {
    let counter = Counter::new();

    {
        let link = Link::<Value>::new();
        let mut rx = Rx::new(&link);
        let mut tx = Tx::new(&link);

        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_empty());

        tx.send_nowait(counter.add(1)).expect("tx.send-nowait");
        assert!(tx.send_nowait(counter.add(2)).expect_err("tx.send-nowait").is_full());

        std::mem::drop(tx);
        assert_eq!(rx.recv_nowait().expect("rx.recv-nowait").unwrap(), 1);
        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_closed());
    }

    assert_eq!(counter.count(), 0);
}

#[test]
fn t_04() {
    let counter = Counter::new();

    {
        let link = Link::<Value>::new();
        let mut rx = Rx::new(&link);
        let mut tx = Tx::new(&link);

        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_empty());
        std::mem::drop(rx);
        assert!(tx.send_nowait(counter.add(1)).expect_err("tx.send-nowait").is_closed());
    }

    assert_eq!(counter.count(), 0);
}

#[test]
fn t_05() {
    let counter = Counter::new();

    {
        let link = Link::<Value>::new();
        let mut rx = Rx::new(&link);
        let mut tx = Tx::new(&link);

        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_empty());

        tx.send_nowait(counter.add(1)).expect("tx.send-nowait");
        assert!(tx.send_nowait(counter.add(2)).expect_err("tx.send-nowait").is_full());

        std::mem::drop(rx);
        assert!(tx.send_nowait(counter.add(3)).expect_err("tx.send-nowait").is_closed());
    }

    assert_eq!(counter.count(), 0);
}

#[tokio::test]
async fn t_06() {
    const ITERATIONS: usize = 1_000_000;

    let counter = Counter::new();
    {
        let link = Link::<Value>::new();

        let producer = async {
            let mut tx = Tx::new(&link);

            let t0 = std::time::Instant::now();
            for i in 0..ITERATIONS {
                tx.send(counter.add(i)).await.expect("tx.send");
                // eprintln!("producer {:?}", i);
            }
            // eprintln!("producer done");
            t0.elapsed()
        };
        let consumer = async {
            let mut rx = Rx::new(&link);

            let mut count = 0;

            let t0 = std::time::Instant::now();
            while let Ok(_v) = rx.recv().await.map(Counted::unwrap) {
                // eprintln!("consumer {:?}", v);
                count += 1;
            }
            // eprintln!("consumer done");
            let dt = t0.elapsed();

            (count, dt)
        };

        let (producer_dt, (count, consumer_dt)) = future::join(producer, consumer).await;

        eprintln!("count:    {:?}", count);
        eprintln!("producer: {:?}", producer_dt);
        eprintln!("consumer: {:?}", consumer_dt);

        assert_eq!(count, ITERATIONS);
    }

    assert_eq!(counter.count(), 0);
}

#[tokio::test]
async fn t_07() {
    const ITERATIONS: usize = 1_000_000;

    let counter = Counter::new();
    {
        let link = Arc::new(Link::<Value>::new());

        let producer = {
            let counter = counter.clone();
            let link = Arc::clone(&link);
            async move {
                let mut tx = Tx::new(link);

                let t0 = std::time::Instant::now();
                for i in 0..ITERATIONS {
                    tx.send(counter.add(i)).await.expect("tx.send");
                    // eprintln!("producer {:?}", i);
                }
                // eprintln!("producer done");
                t0.elapsed()
            }
        };
        let consumer = {
            let link = Arc::clone(&link);
            async move {
                let mut rx = Rx::new(link);

                let mut count = 0;

                let t0 = std::time::Instant::now();
                while let Ok(_v) = rx.recv().await.map(Counted::unwrap) {
                    // eprintln!("consumer {:?}", v);
                    count += 1;
                }
                // eprintln!("consumer done");
                let dt = t0.elapsed();

                (count, dt)
            }
        };

        let producer = tokio::spawn(producer);
        let consumer = tokio::spawn(consumer);

        let producer_dt = producer.await.expect("producer.join");
        let (count, consumer_dt) = consumer.await.expect("cosnumer.join");

        eprintln!("count:    {:?}", count);
        eprintln!("producer: {:?}", producer_dt);
        eprintln!("consumer: {:?}", consumer_dt);

        assert_eq!(count, ITERATIONS);
    }

    assert_eq!(counter.count(), 0);
}

#[test]
#[should_panic]
fn t_08() {
    let link = Link::<Value>::new();
    let mut _rx_1 = Rx::new(&link);
    let mut _rx_2 = Rx::new(&link);
}

#[test]
#[should_panic]
fn t_09() {
    let link = Link::<Value>::new();
    let mut _tx_1 = Tx::new(&link);
    let mut _tx_2 = Tx::new(&link);
}

#[test]
fn t_10() {
    let counter = Counter::new();
    {
        let link = Link::<Value>::new();
        let mut tx = Tx::new(&link);
        let mut rx = Rx::new(&link);

        tx.close();
        assert!(rx.recv_nowait().expect_err("rx.recv_nowait").is_closed());
    }
    assert_eq!(counter.count(), 0);
}

#[test]
fn t_11() {
    let counter = Counter::new();
    {
        let link = Link::<Value>::new();
        let mut tx = Tx::new(&link);
        let mut rx = Rx::new(&link);

        tx.send_nowait(counter.add(1)).expect("tx.send-nowait");
        tx.close();
        assert_eq!(rx.recv_nowait().expect("rx.recv-nowait").unwrap(), 1);
        assert!(rx.recv_nowait().expect_err("rx.recv-nowait").is_closed());
    }
    assert_eq!(counter.count(), 0);
}

#[test]
fn t_12() {
    let counter = Counter::new();
    {
        let link = Link::<Value>::new();
        let mut tx = Tx::new(&link);
        let mut rx = Rx::new(&link);

        rx.close();
        assert!(tx.send_nowait(counter.add(1)).expect_err("tx.send-nowait").is_closed());
    }
    assert_eq!(counter.count(), 0);
}

#[test]
fn t_13() {
    let counter = Counter::new();
    {
        let link = Link::<Value>::new();
        let mut tx = Tx::new(&link);
        let mut rx = Rx::new(&link);

        tx.send_nowait(counter.add(1)).expect("tx.send-nowait");
        rx.close();
        assert!(tx.send_nowait(counter.add(2)).expect_err("tx.send-nowait").is_closed());
    }
    assert_eq!(counter.count(), 0);
}
