use core::borrow::{Borrow, BorrowMut};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct Counter(Arc<()>, Arc<()>);

#[derive(Debug)]
pub struct Counted<T>(Arc<()>, T);

impl Counter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn count(&self) -> usize {
        Arc::strong_count(&self.0) - Arc::strong_count(&self.1)
    }

    pub fn add<T>(&self, value: T) -> Counted<T> {
        Counted(Arc::clone(&self.0), value)
    }
}

impl<T> Counted<T> {
    pub fn unwrap(self) -> T {
        self.1
    }
}

impl<T> Borrow<T> for Counted<T> {
    fn borrow(&self) -> &T {
        &self.1
    }
}

impl<T> BorrowMut<T> for Counted<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.1
    }
}
