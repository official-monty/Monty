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
        self.lock.write_locked.store(false, Ordering::SeqCst);
    }
}

impl WriteGuard<'_> {
    pub fn val(&self) -> NodePtr {
        NodePtr::from_raw(self.lock.value.load(Ordering::SeqCst))
    }

    pub fn store(&self, val: NodePtr) {
        self.lock.value.store(val.inner(), Ordering::SeqCst)
    }
}

impl CustomLock {
    pub fn new(val: NodePtr) -> Self {
        Self { value: AtomicU32::new(val.inner()), write_locked: AtomicBool::new(false) }
    }

    pub fn read(&self) -> NodePtr {
        while self.write_locked.load(Ordering::SeqCst) {
            std::hint::spin_loop();
        }

        NodePtr::from_raw(self.value.load(Ordering::SeqCst))
    }

    pub fn write(&self) -> WriteGuard<'_> {
        while self.write_locked.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            std::hint::spin_loop();
        }

        WriteGuard { lock: self }
    }
}
