use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::{RwLock, atomic::{AtomicUsize, AtomicBool, Ordering} },
};

use tracing::debug;
use loom::sync::CausalCell;

const BUF_SIZE: usize = 256;

struct Slot<T> {
    value: CausalCell<MaybeUninit<T>>,
}

impl<T> Slot<T>
where
    T: Send,
{
    pub fn init() -> Self {
        Slot {
            value: CausalCell::new(MaybeUninit::uninit()),
        }
    }

    /// Copies the providied value into the slot
    ///
    /// Safety: We do not care about the previous contents of this slot, so overwriting the value
    /// inside is safe.
    pub fn write(&self, value: T) {
        self.value.with_mut(|cell| {
            unsafe {
                (*cell).as_mut_ptr().write(value);
            }
        });
    }

    /// Read the value stored in this slot
    ///
    /// Safety: A slot can only be read if it has been previously
    /// written to. Not holding this invarient is undefined behaviour.
    pub fn read(&self) -> &T {
        self.value.with(|value| {
            unsafe {
                &*value.as_ref().unwrap_or_else(|| panic!("We have a Null pointer in a slot!")).as_ptr()
            }
        })
    }
}

pub struct Buffer<T> {
    buf: Box<[Slot<T>]>,
    head: AtomicUsize,
    tail: AtomicUsize,
    size: usize,
}

impl<T> Buffer<T>
where
    T: Send,
{
    pub fn new(size: usize) -> Self {
        let mut buf = Vec::new();

        for _ in 0..=size {
            let slot = Slot::init();
            buf.push(slot);
        }

        Buffer {
            buf: buf.into_boxed_slice(),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(1),
            size,
        }
    }

    pub fn push(&self, value: T) -> Option<()> {
        loop {
            let head = self.head.load(Ordering::SeqCst);
            let tail = self.head.load(Ordering::SeqCst);

            debug!(head, tail);

            let next_idx = head + 1 % self.size;

            if next_idx != tail % BUF_SIZE {
                if self.head.compare_and_swap(head, next_idx, Ordering::SeqCst) == head {
                    // Now that the head is updated, we actually fill the slot
                    self.buf[next_idx].write(value);
                    return Some(());
                } else {
                    continue;
                }
            } else {
                return None;
            }
        }
    }

    pub fn pop(&self) -> Option<&T> {
        loop {
            let head = self.head.load(Ordering::SeqCst);
            let tail = self.head.load(Ordering::SeqCst);

            debug!(message = "popping from queue", head, tail);

            // If there are no elements in the queue, just return early. `insert` ensures that `head`
            // and `tail` never equal each other except when the queue is empty.
            if head == tail {
                return None;
            }

            // It's safe to read from the tail as there is at least one element in thq queue.
            let value = self.buf[tail].read();

            let next_idx = tail + 1 % self.size;

            debug!(message = "Next tail", next_idx);

            if self.tail.compare_and_swap(tail, next_idx, Ordering::SeqCst) == tail {
                return Some(value);
            }
        }
    }
}

unsafe impl<T> Sync for Buffer<T> {}
unsafe impl<T> Send for Buffer<T> {}
