use core::{
    alloc::GlobalAlloc,
    num::NonZeroUsize,
    ptr::{self, NonNull},
};

use talc::{InitOnOom, Span, Talc};

use crate::{
    address_space::{self, AddrSpace, KernelAddrSpaceNotInitializedError},
    spinlock::Spinlock,
};

#[derive(Debug)]
pub enum InitGlobalAllocError {
    UninitKernelAddressSpace(KernelAddrSpaceNotInitializedError),
    AllocError(address_space::AllocError),
}

impl From<KernelAddrSpaceNotInitializedError> for InitGlobalAllocError {
    fn from(v: KernelAddrSpaceNotInitializedError) -> Self {
        Self::UninitKernelAddressSpace(v)
    }
}

impl From<address_space::AllocError> for InitGlobalAllocError {
    fn from(v: address_space::AllocError) -> Self {
        Self::AllocError(v)
    }
}

#[global_allocator]
static ALLOCATOR: TalcWrapper = TalcWrapper {
    inner: Spinlock::new(None),
};

pub unsafe fn init() -> Result<(), InitGlobalAllocError> {
    const PAGES: usize = 10000;

    let addr_space = AddrSpace::kernel();
    let memory = addr_space.allocate(NonZeroUsize::new(10000).unwrap())?;
    let size = 10000 * 4096;
    let span = Span::from_base_size(memory.as_ptr(), size);

    let talc = Talc::new(InitOnOom::new(span));

    ALLOCATOR.inner.lock(|slot| {
        assert!(slot.is_none());
        *slot = Some(talc)
    });
    Ok(())
}

#[derive(Debug)]
struct TalcWrapper {
    inner: Spinlock<Option<Talc<InitOnOom>>>,
}

unsafe impl GlobalAlloc for TalcWrapper {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.inner.lock(|talc| {
            if let Some(t) = talc {
                t.malloc(layout)
                    .map(|p| p.as_ptr())
                    .unwrap_or(ptr::null_mut())
            } else {
                ptr::null_mut()
            }
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if let Some(ptr) = NonNull::new(ptr) {
            self.inner.lock(|talc| {
                if let Some(a) = talc {
                    a.free(ptr, layout);
                }
            })
        }
    }
}
