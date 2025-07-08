use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::NodePtr;

#[derive(Debug)]
pub struct CustomLock {
    value: AtomicU32,
    write_locked: AtomicBool,
}

pub struct WriteGuard<'a> {
    lock: &'a CustomLock,
}

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        // release the write lock with a Release store so subsequent reads
        // see any writes performed while the lock was held
        self.lock.write_locked.store(false, Ordering::Release);
    }
}

impl WriteGuard<'_> {
    pub fn val(&self) -> NodePtr {
        // load the value using Acquire to synchronise with the writer
        NodePtr::from_raw(self.lock.value.load(Ordering::Acquire))
    }

    pub fn store(&self, val: NodePtr) {
        // writes are relaxed as mutual exclusion is provided by the lock
        self.lock.value.store(val.inner(), Ordering::Relaxed)
    }
}

impl CustomLock {
    pub fn new(val: NodePtr) -> Self {
        Self {
            value: AtomicU32::new(val.inner()),
            write_locked: AtomicBool::new(false),
        }
    }

    pub fn read(&self) -> NodePtr {
        // spin until no writer holds the lock
        while self.write_locked.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        NodePtr::from_raw(self.value.load(Ordering::Acquire))
    }

    pub fn write(&self) -> WriteGuard<'_> {
        while self
            .write_locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }

        WriteGuard { lock: self }
    }
}
