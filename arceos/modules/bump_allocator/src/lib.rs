#![no_std]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::ptr::NonNull;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
    count: usize,
}

#[inline]
const fn align_down(pos: usize, align: usize) -> usize {
    pos & !(align - 1)
}

#[inline]
const fn align_up(pos: usize, align: usize) -> usize {
    (pos + align - 1) & !(align - 1)
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            count: 0,
        }
    }

    #[inline]
    const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        // todo!()
        assert!(SIZE > 0);
        assert!(size > 0);
        let end = start.checked_add(size).expect("allocator region overflow");
        self.start = start;
        self.end = end;
        self.b_pos = start;
        self.p_pos = end;
        self.count = 0;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // todo!()
        if size == 0 {
            return Ok(());
        }

        let end = start.checked_add(size).ok_or(AllocError::InvalidParam)?;
        if self.is_empty() {
            self.init(start, size);
            return Ok(());
        }

        if end <= self.start || start >= self.end {
            if end == self.start && self.b_pos == self.start {
                self.start = start;
                self.b_pos = start;
            } else if start == self.end && self.p_pos == self.end {
                self.end = end;
                self.p_pos = end;
            }
            return Ok(());
        }

        Err(AllocError::MemoryOverlap)
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        // todo!()
        let addr = align_up(self.b_pos, layout.align());
        let size = layout.size().max(1);
        let end = addr.checked_add(size).ok_or(AllocError::InvalidParam)?;

        if end > self.p_pos {
            return Err(AllocError::NoMemory);
        }

        self.b_pos = end;
        self.count += 1;
        NonNull::new(addr as *mut u8).ok_or(AllocError::NoMemory)
    }

    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        // todo!()
        if self.count == 0 {
            return;
        }

        self.count -= 1;
        if self.count == 0 {
            self.b_pos = self.start;
        }
    }

    fn total_bytes(&self) -> usize {
        // todo!()
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        // todo!()
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        // todo!()
        self.p_pos.saturating_sub(self.b_pos)
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        // todo!()
        if num_pages == 0 {
            return Err(AllocError::InvalidParam);
        }

        let align = align_pow2.max(SIZE);
        if !align.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }

        let size = num_pages
            .checked_mul(SIZE)
            .ok_or(AllocError::InvalidParam)?;
        let start = self.p_pos.checked_sub(size).ok_or(AllocError::NoMemory)?;
        let start = align_down(start, align);

        if start < self.b_pos {
            return Err(AllocError::NoMemory);
        }

        self.p_pos = start;
        Ok(start)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // todo!()
        // Page allocations are monotonic in this early allocator.
    }

    fn total_pages(&self) -> usize {
        // todo!()
        (self.end - self.start) / SIZE
    }

    fn used_pages(&self) -> usize {
        // todo!()
        (self.end - self.p_pos) / SIZE
    }

    fn available_pages(&self) -> usize {
        // todo!()
        self.available_bytes() / SIZE
    }
}
