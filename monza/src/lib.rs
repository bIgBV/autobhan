use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ptr,
    sync::{
        atomic::{self, AtomicBool, AtomicU32, Ordering},
        RwLock,
    },
};

use loom::sync::CausalCell;
use tracing::debug;

const BUF_SIZE: usize = 256;

struct Slot<T> {
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> Slot<T>
where
    T: Send,
{
    pub fn init() -> Self {
        Slot {
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    // Copies the providied value into the slot
    //
    // Safety: We do not care about the previous contents of this slot, so overwriting the value
    // inside is safe.
    // pub fn write(&self, value: T) {
    //     self.value.with_mut(|cell| unsafe {
    //         (*cell).as_mut_ptr().write(value);
    //     });
    // }

    // Read the value stored in this slot
    //
    // Safety: A slot can only be read if it has been previously
    // written to. Not holding this invarient is undefined behaviour.
    // pub unsafe fn read(&self) -> T {
    //     let cell = self.value.with(|value| ptr::read(value));

    //     cell.assume_init()
    // }
}

pub struct Buffer<T> {
    buf: Box<[Slot<T>]>,
    head: AtomicU32,
    tail: AtomicU32,
    size: usize,
    mask: usize,
}

impl<T> Buffer<T>
where
    T: Send + Clone,
{
    pub fn new(size: usize) -> Self {
        let mut buf = Vec::new();

        for _ in 0..=size {
            let slot = Slot::init();
            buf.push(slot);
        }

        Buffer {
            buf: buf.into_boxed_slice(),
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            size,
            mask: size - 1,
        }
    }

    pub fn push(&self, value: T) {
        loop {
            let head = self.head.load(Ordering::SeqCst);
            let tail = self.head.load(Ordering::SeqCst);

            debug!(head, tail);

            if tail.wrapping_sub(head) < self.size as u32 {
                let idx = tail as usize & self.mask;

                unsafe {
                    let ptr = self.buf[idx].value.get();
                    // write the value to the slot
                    ptr::write((*ptr).as_mut_ptr(), value.clone())
                };

                let current = tail;

                let actual =
                    self.tail
                        .compare_and_swap(tail, tail.wrapping_add(1), Ordering::SeqCst);

                debug!(?self.head, ?self.tail);

                if actual == current {
                    return;
                }
            }

            atomic::spin_loop_hint();
        }
    }

    pub fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::SeqCst);
            let tail = self.tail.load(Ordering::SeqCst);

            debug!(message = "popping from queue", head, tail);

            // If there are no elements in the queue, just return early. `insert` ensures that `head`
            // and `tail` never equal each other except when the queue is empty.
            if head == tail {
                return None;
            }

            let idx = head as usize & self.mask;

            // It's safe to read from the tail as there is at least one element in thq queue.
            let value = unsafe {
                let ptr = self.buf[idx].value.get();
                ptr::read(ptr)
            };

            let actual = self
                .head
                .compare_and_swap(head, head.wrapping_add(1), Ordering::SeqCst);

            debug!(message = "updated", ?self.head, ?self.tail);

            if actual == head {
                return Some(unsafe { value.assume_init() });
            }

            // lost the race, try again
            atomic::spin_loop_hint()
        }
    }
}

unsafe impl<T> Sync for Buffer<T> {}
unsafe impl<T> Send for Buffer<T> {}
