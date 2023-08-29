use core::{ops::Range, slice};

use limine::{MemmapEntry, MemmapRequest, MemoryMapEntryType, NonNullPtr};

use crate::{
    hhdm::{Hhdm, HigherHalf},
    spinlock::Spinlock,
    types::{Frame, PhysAddr},
};

static GLOBAL: Spinlock<Option<GlobalInner>> = Spinlock::new(None);

#[derive(Debug, Default, Clone, Copy)]
pub struct Global;

unsafe impl PhysicalMemoryAllocator for Global {
    fn allocate_frame(&self) -> Result<Frame, PhysAllocError> {
        let frame = GLOBAL.lock(|global| {
            let global = match global {
                Some(v) => v,
                None => {
                    let inner = GlobalInner::with_limine().ok_or(PhysAllocError)?;
                    global.insert(inner)
                }
            };

            if let Some(frame) = global.freelist_pop() {
                return Ok(frame);
            }
            global.memmap_pop().ok_or(PhysAllocError)
        });
        if let Ok(frame) = frame {
            log::trace!("allocated frame {:#x?}", frame);
        }
        frame
    }

    unsafe fn deallocate_frame(&self, frame: Frame) {
        GLOBAL.lock(|global| {
            let global = global.as_mut().expect("deallocation prior to pmm init");
            global.freelist_push(frame);
        });
    }
}

#[derive(Debug)]
pub struct PhysAllocError;

pub unsafe trait PhysicalMemoryAllocator {
    fn allocate_frame(&self) -> Result<Frame, PhysAllocError>;
    unsafe fn deallocate_frame(&self, frame: Frame);
}

struct GlobalInner {
    hhdm: Hhdm,
    free: Option<HigherHalf<Node>>,
    current: Range<u64>,
    entries: slice::Iter<'static, NonNullPtr<MemmapEntry>>,
}

unsafe impl Send for GlobalInner {}

impl GlobalInner {
    pub fn with_limine() -> Option<Self> {
        static REQUEST: MemmapRequest = MemmapRequest::new(0);
        let response = REQUEST.get_response().get()?;
        let entries = response.memmap().iter();

        Some(Self {
            hhdm: Hhdm::with_limine(),
            free: None,
            current: 0..0,
            entries,
        })
    }

    fn memmap_pop(&mut self) -> Option<Frame> {
        while (self.current.end - self.current.start) < 4096 {
            let entry = self.entries.next()?;
            if entry.typ != MemoryMapEntryType::Usable {
                continue;
            }
            self.current = entry.base..entry.base + entry.len;
        }

        let addr = PhysAddr(self.current.start);
        self.current.start += 4096;
        Some(Frame(addr))
    }

    fn freelist_pop(&mut self) -> Option<Frame> {
        let head = self.free.take()?;
        self.free = unsafe { (*head.as_ptr()).next };
        let phys = self.hhdm.to_physical(head);
        Some(Frame(phys))
    }

    unsafe fn freelist_push(&mut self, frame: Frame) {
        let ptr: HigherHalf<Node> = self.hhdm.to_virtual(frame.0);
        unsafe {
            (*ptr.as_ptr()).next = self.free;
        }
        self.free = Some(ptr);
    }
}

#[repr(C, align(4096))]
#[derive(Debug, Default)]
struct Node {
    next: Option<HigherHalf<Node>>,
}
