use std::{
    mem::{copy_nonoverlapping, MaybeUninit},
    sync::atomic::{AtomicUsize, Ordering},
};

const BUF_SIZE: usize = 256;

pub struct Slot<T> {
    value: MaybeUninit<T>,
}

impl<T> Slot<T>
where
    T: Send,
{
    pub fn init() -> Self {
        Slot {
            value: MaybeUninit::zeroed(),
        }
    }

    /// Copies the providied value into the slot
    ///
    ///
    pub fn write(&mut self, value: &T) {
        unsafe {
            let src_ptr: *const T = value;
            let dst_ptr: *mut T = self.value.as_mut_ptr();

            copy_nonoverlapping(src_ptr, dst_prt, 1);
        }
    }

    /// Read the value stored in this slot
    ///
    /// **Safety**: A slot can only be read if it has been previously
    /// written to. Not holding this invarient is undefined behaviour.
    pub unsafe fn read(mut self) -> T {
        let value = self.value.assume_init();
        self.value = MaybeUninit::zeroed();
        value
    }
}

pub struct Buffer<T> {
    buf: Box<[Slot<T>]>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T> Buffer<T>
where
    T: Send,
{
    pub fn new() -> Self {
        let mut buf = Vec::new();

        for _i in 0..BUF_SIZE {
            let slot = Slot::init();
            buf.push(slot);
        }

        Buffer {
            buf: buf.into_boxed_slice(),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(1),
        }
    }

    pub fn insert(&mut self, value: &T) -> Option<()> {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.head.load(Ordering::SeqCst);

        let next_idx = head + 1 % BUF_SIZE;

        if next_idx < tail % BUF_SIZE {
            unsafe { self.buf[next_idx].write(value) }

            loop {
                if self.head.compare_and_swap(head, next_idx, Ordering::SeqCst) == head {
                    break;
                }
            }

            Some(())
        }

        None
    }
}
