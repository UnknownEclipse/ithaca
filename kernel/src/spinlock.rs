use spin::mutex::SpinMutex;

use crate::interrupts;

#[derive(Debug)]
pub struct Spinlock<T> {
    mutex: SpinMutex<T>,
}

impl<T> Spinlock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            mutex: SpinMutex::new(value),
        }
    }

    pub fn lock<F, U>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        interrupts::without(|| {
            let mut guard = self.mutex.lock();
            f(&mut *guard)
        })
    }
}
