use core::{iter::Step, mem, num::NonZeroUsize, ops::Range, ptr::NonNull};

use bytemuck::TransparentWrapper;

use self::x86_64::PageMapper;
use crate::{
    address_space::x86_64::{MapError, PageFlags},
    boot::KERNEL_ADDRESS_REQUEST,
    pmm::{self, PhysAllocError, PhysicalMemoryAllocator},
    spinlock::Spinlock,
    types::{Frame, Page, VirtAddr},
    vmm::{self, VirtAllocError, VirtualRegionAllocator},
};

mod x86_64;

#[derive(Debug)]
pub enum AllocError {
    PhysAllocError(PhysAllocError),
    VirtAllocError(VirtAllocError),
}

impl From<PhysAllocError> for AllocError {
    fn from(value: PhysAllocError) -> Self {
        Self::PhysAllocError(value)
    }
}

impl From<VirtAllocError> for AllocError {
    fn from(value: VirtAllocError) -> Self {
        Self::VirtAllocError(value)
    }
}

#[derive(Debug)]
pub enum MapFramesError {
    PhysAllocError(PhysAllocError),
    VirtAllocError(VirtAllocError),
}

impl From<PhysAllocError> for MapFramesError {
    fn from(value: PhysAllocError) -> Self {
        Self::PhysAllocError(value)
    }
}

impl From<VirtAllocError> for MapFramesError {
    fn from(value: VirtAllocError) -> Self {
        Self::VirtAllocError(value)
    }
}

#[derive(Debug, Default)]
pub struct MapOptions {
    pub user: bool,
    pub writable: bool,
    pub disable_cache: bool,
}

#[derive(Debug)]
pub struct KernelAddrSpaceNotInitializedError;

#[repr(transparent)]
#[derive(Debug, TransparentWrapper)]
pub struct AddrSpace {
    inner: AddrSpaceInner,
}

impl AddrSpace {
    pub fn kernel() -> &'static AddrSpace {
        TransparentWrapper::wrap_ref(&AddrSpaceInner::Kernel)
    }

    pub unsafe fn handle_page_fault(&self, _addr: VirtAddr) {
        match &self.inner {
            AddrSpaceInner::Kernel => unreachable!("page fault in kernel code"),
        }
    }

    pub fn map_frames(
        &self,
        frames: Range<Frame>,
        map_options: MapOptions,
    ) -> Result<NonNull<u8>, AllocError> {
        match &self.inner {
            AddrSpaceInner::Kernel => KernelAddrSpace.map_frames(frames, map_options),
        }
    }

    pub fn allocate(&self, pages: NonZeroUsize) -> Result<NonNull<u8>, AllocError> {
        match &self.inner {
            AddrSpaceInner::Kernel => KernelAddrSpace.allocate(pages),
        }
    }
}

#[derive(Debug, Clone)]
enum AddrSpaceInner {
    Kernel,
}

#[derive(Debug, Default, Clone, Copy)]
struct KernelAddrSpace;

impl KernelAddrSpace {
    pub fn allocate(&mut self, pages: NonZeroUsize) -> Result<NonNull<u8>, AllocError> {
        with_kernel_address_space(|inner| inner.allocate(pages))
    }

    pub fn map_frames(
        &self,
        frames: Range<Frame>,
        map_options: MapOptions,
    ) -> Result<NonNull<u8>, AllocError> {
        with_kernel_address_space(|inner| inner.map_frames(frames, map_options))
    }
}

fn with_kernel_address_space<F, T>(f: F) -> T
where
    F: FnOnce(&mut KernelAddrSpaceInner) -> T,
{
    KERNEL.lock(|slot| f(slot.get_or_insert_with(KernelAddrSpaceInner::with_limine)))
}

static KERNEL: Spinlock<Option<KernelAddrSpaceInner>> = Spinlock::new(None);

struct FrameDropGuard<'a, P>
where
    P: PhysicalMemoryAllocator,
{
    frame: Frame,
    pmm: &'a P,
}

impl<'a, P> Drop for FrameDropGuard<'a, P>
where
    P: PhysicalMemoryAllocator,
{
    fn drop(&mut self) {
        unsafe { self.pmm.deallocate_frame(self.frame) };
    }
}

#[derive(Debug)]
struct KernelAddrSpaceInner {
    vmm: vmm::BumpAllocator,
    pmm: pmm::Global,
    mapper: PageMapper,
}

impl KernelAddrSpaceInner {
    pub fn with_limine() -> Self {
        let kernel_address = KERNEL_ADDRESS_REQUEST.get_response().get().unwrap();

        let start = VirtAddr(usize::MAX.wrapping_shl(47));
        let end = VirtAddr(kernel_address.virtual_base as usize);

        assert!(start <= end);

        Self {
            vmm: vmm::BumpAllocator::new(Page(start)..Page(end)),
            pmm: pmm::Global,
            mapper: unsafe { PageMapper::active() },
        }
    }

    pub fn allocate(&mut self, pages: NonZeroUsize) -> Result<NonNull<u8>, AllocError> {
        struct DeallocRegion<'a> {
            region: Range<Page>,
            pmm: &'a pmm::Global,
            mapper: &'a mut PageMapper,
        }

        impl<'a> Drop for DeallocRegion<'a> {
            fn drop(&mut self) {
                for page in self.region.clone() {
                    unsafe {
                        let frame = self.mapper.unmap_page(page).expect("failed to unmap page");
                        self.pmm.deallocate_frame(frame);
                    }
                }
            }
        }

        let pages = self.vmm.allocate_region(pages)?;

        let mut region_guard = DeallocRegion {
            mapper: &mut self.mapper,
            pmm: &self.pmm,
            region: pages.start..pages.start,
        };

        for page in pages.clone() {
            let frame = self.pmm.allocate_frame()?;

            let frame_drop_guard = FrameDropGuard {
                frame,
                pmm: &self.pmm,
            };

            let result = unsafe {
                region_guard.mapper.map_page(
                    page,
                    frame,
                    PageFlags::PRESENT | PageFlags::WRITABLE,
                    &self.pmm,
                )
            };

            match result {
                Ok(_) => {
                    mem::forget(frame_drop_guard);
                    region_guard.region.end = Step::forward(page, 1);
                    assert_eq!(region_guard.mapper.translate_page(page), Some(frame));
                }
                Err(MapError::PhysAllocError(err)) => {
                    return Err(AllocError::PhysAllocError(err));
                }
                Err(MapError::PageAlreadyMapped) => {
                    // This should never occur so long as the virtual address allocator
                    // is functioning correctly.
                    unreachable!("attempted to map previously mapped page")
                }
            }
        }

        mem::forget(region_guard);
        let ptr = pages.start.0.as_ptr().cast();
        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }

    pub fn map_frames(
        &mut self,
        frames: Range<Frame>,
        map_options: MapOptions,
    ) -> Result<NonNull<u8>, AllocError> {
        let n = Step::steps_between(&frames.start, &frames.end)
            .expect("invalid physical memory region");
        let Some(pages) = NonZeroUsize::new(n) else {
            return Ok(NonNull::dangling());
        };
        let pages = self.vmm.allocate_region(pages)?;

        let mut flags = PageFlags::PRESENT;
        if map_options.writable {
            flags |= PageFlags::WRITABLE;
        }
        if map_options.user {
            flags |= PageFlags::USER;
        }
        if map_options.disable_cache {
            flags |= PageFlags::DISABLE_CACHE;
        }
        for (page, frame) in pages.clone().zip(frames) {
            match unsafe { self.mapper.map_page(page, frame, flags, &self.pmm) } {
                Ok(_) => {}
                Err(MapError::PageAlreadyMapped) => panic!("page already mapped"),
                Err(MapError::PhysAllocError(err)) => return Err(err.into()),
            }
        }

        Ok(unsafe { NonNull::new_unchecked(pages.start.0.as_ptr().cast()) })
    }
}
