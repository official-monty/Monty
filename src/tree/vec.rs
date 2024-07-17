use std::{
    alloc::{self, Layout}, sync::atomic::{AtomicPtr, AtomicU16, Ordering}
};

use super::Edge;

const EDGE_SIZE: usize = std::mem::size_of::<Edge>();
const EDGE_ALIGN: usize = std::mem::align_of::<Edge>();

#[derive(Debug)]
pub struct AtomicVec {
    ptr: AtomicPtr<Edge>,
    len: AtomicU16,
    cap: AtomicU16,
}

impl Drop for AtomicVec {
    fn drop(&mut self) {
        self.dealloc();
    }
}

impl AtomicVec {
    pub fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(std::ptr::null_mut()),
            len: AtomicU16::new(0),
            cap: AtomicU16::new(0),
        }
    }

    pub fn alloc(&self, len: usize) {
        if self.cap() > len {
            self.len.store(len as u16, Ordering::Relaxed);
            return;
        }

        if len == 0 {
            return;
        }

        self.dealloc();

        self.len.store(len as u16, Ordering::Relaxed);
        self.cap.store(len as u16, Ordering::Relaxed);

        let layout = Layout::from_size_align(EDGE_SIZE * self.cap(), EDGE_ALIGN).unwrap();

        unsafe {
            let ptr = alloc::alloc(layout);
            self.ptr.store(ptr.cast(), Ordering::Relaxed);
        }
    }

    pub fn dealloc(&self) {
        let ptr = self.ptr();

        if ptr.is_null() {
            return;
        }

        let layout = Layout::from_size_align(EDGE_SIZE * self.cap(), EDGE_ALIGN).unwrap();

        self.ptr.store(std::ptr::null_mut(), Ordering::Relaxed);
        self.len.store(0, Ordering::Relaxed);
        self.cap.store(0, Ordering::Relaxed);

        unsafe {
            alloc::dealloc(ptr.cast(), layout);
        }
    }

    pub fn clear(&self) {
        self.len.store(0, Ordering::Relaxed);
    }

    fn ptr(&self) -> *mut Edge {
        self.ptr.load(Ordering::Relaxed)
    }

    fn cap(&self) -> usize {
        usize::from(self.cap.load(Ordering::Relaxed))
    }

    pub fn len(&self) -> usize {
        usize::from(self.len.load(Ordering::Relaxed))
    }

    pub fn elements(&self) -> &[Edge] {
        let ptr = self.ptr.load(Ordering::Relaxed);

        if ptr.is_null() {
            return &[];
        }

        unsafe {
            std::slice::from_raw_parts(ptr, self.len())
        }
    }
}