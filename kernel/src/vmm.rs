use core::{cell::Cell, iter::Step, num::NonZeroUsize, ops::Range};

use atomic::{Atomic, Ordering};

use crate::types::Page;

#[derive(Debug)]
pub enum VirtAllocError {
    VirtualAddressSpaceExhausted,
}

pub unsafe trait VirtualRegionAllocator {
    fn allocate_region(&self, pages: NonZeroUsize) -> Result<Range<Page>, VirtAllocError>;
}

pub unsafe trait VirtualRegionDeallocator {
    unsafe fn deallocate_region(&self, region: Range<Page>);
}

#[derive(Debug)]
pub struct BumpAllocator {
    full: Range<Page>,
    pos: Cell<Page>,
}

impl BumpAllocator {
    pub fn new(full: Range<Page>) -> Self {
        Self {
            full: full.clone(),
            pos: Cell::new(full.start),
        }
    }
}

unsafe impl VirtualRegionAllocator for BumpAllocator {
    fn allocate_region(&self, pages: NonZeroUsize) -> Result<Range<Page>, VirtAllocError> {
        let start = self.pos.get();
        let end = Step::forward_checked(start, pages.get())
            .ok_or(VirtAllocError::VirtualAddressSpaceExhausted)?;
        if self.full.end < end {
            return Err(VirtAllocError::VirtualAddressSpaceExhausted);
        }
        self.pos.set(end);
        Ok(start..end)
    }
}

#[derive(Debug)]
pub struct SyncBumpAllocator {
    full: Range<Page>,
    pos: Atomic<Page>,
}

impl SyncBumpAllocator {
    pub fn new(full: Range<Page>) -> Self {
        Self {
            full: full.clone(),
            pos: Atomic::new(full.start),
        }
    }
}

unsafe impl VirtualRegionAllocator for SyncBumpAllocator {
    fn allocate_region(&self, pages: NonZeroUsize) -> Result<Range<Page>, VirtAllocError> {
        let mut start = self.pos.load(Ordering::Acquire);

        loop {
            let end = Step::forward_checked(start, pages.get())
                .ok_or(VirtAllocError::VirtualAddressSpaceExhausted)?;
            if self.full.end < end {
                return Err(VirtAllocError::VirtualAddressSpaceExhausted);
            }

            if let Err(err) =
                self.pos
                    .compare_exchange_weak(start, end, Ordering::AcqRel, Ordering::Acquire)
            {
                start = err;
                continue;
            }
            return Ok(start..end);
        }
    }
}
