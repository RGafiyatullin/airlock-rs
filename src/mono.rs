use core::borrow::Borrow;
use core::future;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU8, Ordering};
use core::task::{Context, Poll};

use futures::task::AtomicWaker;

use crate::error::{RecvError, RecvErrorNoWait, SendError, SendErrorNoWait};
use crate::slot::Slot;

const FLAG_IS_CLOSED: u8 = 0b0001;
const FLAG_IS_FULL: u8 = 0b0010;
const FLAG_TX_IS_SET: u8 = 0b0100;
const FLAG_RX_IS_SET: u8 = 0b1000;
const ATOMIC_UPDATE_MAX_ITERATIONS: usize = 1024;

pub struct Link<T> {
    flags: AtomicU8,
    rx_waker: AtomicWaker,
    tx_waker: AtomicWaker,
    slot: Slot<T>,
}

pub struct Rx<T, L>
where
    L: Borrow<Link<T>>,
{
    link: L,
    _value: PhantomData<T>,
}

pub struct Tx<T, L>
where
    L: Borrow<Link<T>>,
{
    link: L,
    _value: PhantomData<T>,
}

impl<T, L> Rx<T, L>
where
    L: Borrow<Link<T>>,
{
    pub fn new(link: L) -> Self {
        link.borrow().set_rx();
        Self { link, _value: Default::default() }
    }

    pub fn recv_nowait(&mut self) -> Result<T, RecvErrorNoWait> {
        self.link.borrow().recv_nowait()
    }

    pub async fn recv(&mut self) -> Result<T, RecvError> {
        let link = self.link.borrow();
        future::poll_fn(|cx| link.poll_recv(cx)).await
    }
}

impl<T, L> Tx<T, L>
where
    L: Borrow<Link<T>>,
{
    pub fn new(link: L) -> Self {
        link.borrow().set_tx();
        Self { link, _value: Default::default() }
    }

    pub fn send_nowait(&mut self, value: T) -> Result<(), SendErrorNoWait<T>> {
        self.link.borrow().send_nowait(value)
    }

    pub async fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let mut value = Some(value);
        let link = self.link.borrow();
        future::poll_fn(|cx| link.poll_send(cx, &mut value)).await
    }
}

impl<T> Link<T> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> Link<T> {
    fn poll_recv(&self, cx: &mut Context) -> Poll<Result<T, RecvError>> {
        self.rx_waker.register(cx.waker());
        match self.recv_nowait() {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(RecvErrorNoWait::Closed) => Poll::Ready(Err(RecvError::closed())),
            Err(RecvErrorNoWait::Empty) => Poll::Pending,
        }
    }

    fn poll_send(&self, cx: &mut Context, value: &mut Option<T>) -> Poll<Result<(), SendError<T>>> {
        self.tx_waker.register(cx.waker());
        match self.send_nowait(value.take().expect("stolen value")) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(SendErrorNoWait::Closed(rejected)) => Poll::Ready(Err(SendError::closed(rejected))),
            Err(SendErrorNoWait::Full(rejected)) => {
                *value = Some(rejected);
                Poll::Pending
            },
        }
    }

    fn recv_nowait(&self) -> Result<T, RecvErrorNoWait> {
        let flags = self.flags.load(Ordering::SeqCst);

        let is_closed = flags & FLAG_IS_CLOSED != 0;
        let is_empty = flags & FLAG_IS_FULL == 0;

        match (is_closed, is_empty) {
            (false, true) => Err(RecvErrorNoWait::empty()),
            (true, true) => Err(RecvErrorNoWait::closed()),

            (_, false) => {
                let value = unsafe { self.slot.as_maybe_uninit_mut().assume_init_read() };

                let mut atomic_update_success = false;

                let mut old_flags = flags;
                for _ in 0..ATOMIC_UPDATE_MAX_ITERATIONS {
                    let new_flags = old_flags & !FLAG_IS_FULL;
                    match self.flags.compare_exchange(
                        old_flags,
                        new_flags,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => {
                            atomic_update_success = true;
                            break
                        },
                        Err(v) => {
                            old_flags = v;
                            continue
                        },
                    }
                }

                if !atomic_update_success {
                    panic!("failed to perform atomic update")
                }

                self.tx_waker.wake();

                Ok(value)
            },
        }
    }

    fn send_nowait(&self, value: T) -> Result<(), SendErrorNoWait<T>> {
        let flags = self.flags.load(Ordering::SeqCst);

        let is_closed = flags & FLAG_IS_CLOSED != 0;
        let is_full = flags & FLAG_IS_FULL != 0;

        match (is_closed, is_full) {
            (true, _) => Err(SendErrorNoWait::closed(value)),
            (false, true) => Err(SendErrorNoWait::full(value)),
            (false, false) => {
                unsafe { self.slot.as_maybe_uninit_mut() }.write(value);
                let mut atomic_update_success = false;
                let mut old_flags = flags;
                for _ in 0..ATOMIC_UPDATE_MAX_ITERATIONS {
                    let new_flags = old_flags | FLAG_IS_FULL;
                    match self.flags.compare_exchange(
                        old_flags,
                        new_flags,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => {
                            atomic_update_success = true;
                            break
                        },
                        Err(v) => {
                            old_flags = v;
                            continue
                        },
                    }
                }

                if !atomic_update_success {
                    panic!("failed to perform atomic update")
                }

                self.rx_waker.wake();

                Ok(())
            },
        }
    }

    fn close(&self, notify_tx: bool, notify_rx: bool) {
        let mut atomic_update_success = false;
        let mut old_flags = self.flags.load(Ordering::SeqCst);
        for _ in 0..ATOMIC_UPDATE_MAX_ITERATIONS {
            let new_flags = old_flags | FLAG_IS_CLOSED;

            match self.flags.compare_exchange(
                old_flags,
                new_flags,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    atomic_update_success = true;
                    break
                },
                Err(v) => {
                    old_flags = v;
                    continue
                },
            }
        }

        if !atomic_update_success {
            panic!("failed to perform atomic update")
        }

        if notify_tx {
            self.tx_waker.wake();
        }
        if notify_rx {
            self.rx_waker.wake();
        }
    }

    fn set_tx(&self) {
        let mut atomic_update_success = false;

        let mut old_flags = self.flags.load(Ordering::SeqCst);

        for _ in 0..ATOMIC_UPDATE_MAX_ITERATIONS {
            if old_flags & FLAG_TX_IS_SET != 0 {
                panic!("this link already has a Tx");
            }

            let new_flags = old_flags | FLAG_TX_IS_SET;

            match self.flags.compare_exchange(
                old_flags,
                new_flags,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    atomic_update_success = true;
                    break
                },
                Err(v) => {
                    old_flags = v;
                    continue
                },
            }
        }

        if !atomic_update_success {
            panic!("failed to perform atomic update")
        }
    }
    fn set_rx(&self) {
        let mut atomic_update_success = false;

        let mut old_flags = self.flags.load(Ordering::SeqCst);

        for _ in 0..ATOMIC_UPDATE_MAX_ITERATIONS {
            if old_flags & FLAG_RX_IS_SET != 0 {
                panic!("this link already has an Rx");
            }

            let new_flags = old_flags | FLAG_RX_IS_SET;

            match self.flags.compare_exchange(
                old_flags,
                new_flags,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    atomic_update_success = true;
                    break
                },
                Err(v) => {
                    old_flags = v;
                    continue
                },
            }
        }

        if !atomic_update_success {
            panic!("failed to perform atomic update")
        }
    }
}

impl<T> Default for Link<T> {
    fn default() -> Self {
        Self {
            flags: Default::default(),
            rx_waker: Default::default(),
            tx_waker: Default::default(),
            slot: Default::default(),
        }
    }
}

impl<T> Drop for Link<T> {
    fn drop(&mut self) {
        if self.flags.load(Ordering::SeqCst) & FLAG_IS_FULL != 0 {
            unsafe {
                self.slot.as_maybe_uninit_mut().assume_init_drop();
            }
        }
    }
}

impl<T, L> Drop for Rx<T, L>
where
    L: Borrow<Link<T>>,
{
    fn drop(&mut self) {
        self.link.borrow().close(/* notify_tx: */ true, /* notify_rx: */ false);
    }
}

impl<T, L> Drop for Tx<T, L>
where
    L: Borrow<Link<T>>,
{
    fn drop(&mut self) {
        self.link.borrow().close(/* notify_tx: */ false, /* notify_rx: */ true);
    }
}
