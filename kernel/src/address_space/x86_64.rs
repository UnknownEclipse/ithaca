use core::{arch::asm, cell::Cell, fmt::Debug, ptr};

use bitflags::bitflags;
use bytemuck::Zeroable;

use crate::{
    hhdm::{Hhdm, HigherHalf},
    pmm::{PhysAllocError, PhysicalMemoryAllocator},
    types::{Frame, Page, PhysAddr, VirtAddr},
    x86_64::cr3,
};

#[derive(Debug)]
pub enum MapError {
    PhysAllocError(PhysAllocError),
    PageAlreadyMapped,
}

#[derive(Debug)]
pub enum UnmapError {
    PageNotMapped,
}

impl From<PhysAllocError> for MapError {
    fn from(value: PhysAllocError) -> Self {
        MapError::PhysAllocError(value)
    }
}

#[derive(Debug)]
pub struct PageMapper {
    l4: HigherHalf<PageTable>,
    hhdm: Hhdm,
}

unsafe impl Send for PageMapper {}

impl PageMapper {
    pub unsafe fn active() -> Self {
        let hhdm = Hhdm::with_limine();
        let frame = cr3::read();
        let l4 = hhdm.to_virtual(frame.0);
        Self { l4, hhdm }
    }

    pub unsafe fn map_page(
        &mut self,
        page: Page,
        frame: Frame,
        flags: PageFlags,
        phys_alloc: &impl PhysicalMemoryAllocator,
    ) -> Result<(), MapError> {
        log::trace!("mapping {:x?} to {:x?}", page, frame);

        let vaddr = page.0.addr();
        let mut page_table = self.l4.as_ref();

        for level in (1..4).rev() {
            let page_table_index = vaddr.wrapping_shr(12 + 9 * level) & 0x1ff;
            let entry_cell = &page_table.entries[page_table_index];
            let mut entry = entry_cell.get();

            if !entry.flags().contains(PageFlags::PRESENT) {
                log::trace!(
                    "found empty l{} page table entry, allocating new page table",
                    level + 1,
                );

                let page_table_frame = phys_alloc.allocate_frame()?;
                let page_table_ptr = self.hhdm.to_virtual(page_table_frame.0);
                ptr::write(page_table_ptr.as_ptr(), PageTable::empty());

                entry = PageTableEntry::new(
                    PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::USER,
                    page_table_frame,
                );
                entry_cell.set(entry);
            } else if entry.flags().contains(PageFlags::HUGE_PAGE) {
                todo!("huge page handling");
            }

            let frame = entry.frame();
            let child_page_table_ptr = self.hhdm.to_virtual(frame.0);
            page_table = child_page_table_ptr.as_ref();
        }

        let page_table_index = vaddr.wrapping_shr(12) & 0x1ff;
        let entry_cell = &page_table.entries[page_table_index];
        let entry = PageTableEntry::new(flags, frame);
        if entry_cell.get().flags().contains(PageFlags::PRESENT) {
            return Err(MapError::PageAlreadyMapped);
        }

        entry_cell.set(entry);
        tlb_flush(page.0);
        Ok(())
    }

    pub unsafe fn unmap_page(&mut self, page: Page) -> Result<Frame, UnmapError> {
        log::trace!("unmapping page {:#x?}", page);
        let slot = self
            .get_entry(page.0.addr())
            .ok_or(UnmapError::PageNotMapped)?;

        let pte = slot.get();
        if !pte.flags().contains(PageFlags::PRESENT) {
            return Err(UnmapError::PageNotMapped);
        }

        let frame = pte.frame();
        let pte = PageTableEntry::new(PageFlags::empty(), frame);
        slot.set(pte);
        tlb_flush(page.0);
        Ok(frame)
    }

    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let entry = self.get_entry(page.0.addr())?.get();

        entry
            .flags()
            .contains(PageFlags::PRESENT)
            .then_some(entry.frame())
    }

    fn get_entry(&self, addr: usize) -> Option<&Cell<PageTableEntry>> {
        let mut page_table = unsafe { self.l4.as_ref() };

        for i in (1..4).rev() {
            let index = addr.wrapping_shr(12 + 9 * i) & 0x1ff;
            let pte = page_table.entries[index].get();

            if !pte.flags().contains(PageFlags::PRESENT) {
                return None;
            }

            let frame = pte.frame();
            let ptr: HigherHalf<PageTable> = self.hhdm.to_virtual(frame.0);
            page_table = unsafe { ptr.as_ref() };
        }

        let index = addr.wrapping_shr(12) & 0x1ff;
        Some(&page_table.entries[index])
    }
}

#[repr(C, align(4096))]
#[derive(Debug, Zeroable)]
struct PageTable {
    entries: [Cell<PageTableEntry>; 512],
}

impl PageTable {
    pub fn empty() -> Self {
        Zeroable::zeroed()
    }
}

const FRAME_MASK: u64 = u64::MAX.wrapping_shl(13).wrapping_shr(1);

#[repr(transparent)]
#[derive(Clone, Copy, Zeroable)]
struct PageTableEntry(u64);

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.flags().contains(PageFlags::PRESENT) {
            f.debug_struct("PageTableEntry")
                .field("flags", &self.flags())
                .field("frame", &self.frame())
                .finish()
        } else {
            f.write_str("<missing>")

            // f.debug_tuple("PageTableEntry").field(&"<missing>").finish()
        }
    }
}

impl PageTableEntry {
    pub fn missing() -> Self {
        Self(0)
    }

    pub fn new(flags: PageFlags, frame: Frame) -> Self {
        let flags = flags.bits() & !FRAME_MASK;
        Self(flags | frame.0 .0)
    }

    pub fn flags(&self) -> PageFlags {
        PageFlags::from_bits_truncate(self.0)
    }

    // pub fn set_frame(&mut self, frame: Frame) {
    //     self.0 &= FRAME_MASK;
    //     self.0 |= frame.0 .0;
    // }

    pub fn frame(&self) -> Frame {
        Frame(PhysAddr(self.0 & FRAME_MASK))
    }

    // pub fn set_flags(&mut self, flags: PageFlags) {
    //     self.0 &= !PageFlags::all().bits();
    //     self.0 |= (flags & PageFlags::all()).bits();
    // }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PageFlags: u64 {
        const PRESENT = 1;
        const WRITABLE = 1 << 1;
        const USER = 1 << 2;
        const DISABLE_CACHE = 1 << 4;
        const HUGE_PAGE = 1 << 7;
    }
}

pub fn tlb_flush(addr: VirtAddr) {
    unsafe { asm!("invlpg [{}]", in(reg) addr.0) }
}

pub fn tlb_nuke() {
    unsafe { asm!("mov {tmp}, cr3", "mov cr3, {tmp}", tmp = out(reg) _) };
}
