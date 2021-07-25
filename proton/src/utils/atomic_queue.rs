use core::sync::atomic::{AtomicPtr, Ordering};
use alloc::boxed::Box;

struct Cell<T> {
    next: AtomicPtr<Cell<T>>,
    data: T,
}

pub struct AtomicQueue<T> {
    head: AtomicPtr<Cell<T>>,
    tail: AtomicPtr<Cell<T>>,
}

impl <T> AtomicQueue<T> {
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(0usize as _),
            tail: AtomicPtr::new(0usize as _),
        }
    }

    pub fn push(&self, t: T) {
        let cell = Box::into_raw(box Cell {
            next: AtomicPtr::default(),
            data: t,
        });
        let old_tail = self.tail.load(Ordering::SeqCst);
        self.tail.store(cell, Ordering::SeqCst);

        if 0usize as *mut Cell<T> != self.head.compare_and_swap(0usize as *mut Cell<T>, cell, Ordering::SeqCst) {
            unsafe {
                (*old_tail).next.store(cell, Ordering::SeqCst);
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        loop {
            let old_head = self.head.load(Ordering::SeqCst);
            if old_head as usize == 0 {
                return None
            }
            let new_head = unsafe { (*old_head).next.load(Ordering::SeqCst) };
            if old_head == self.head.compare_and_swap(old_head, new_head, Ordering::SeqCst) {
                self.tail.compare_and_swap(old_head, new_head, Ordering::SeqCst);
                let boxed_cell = unsafe { Box::from_raw(old_head) };
                return Some(boxed_cell.data);
            }
        }
    }
}
