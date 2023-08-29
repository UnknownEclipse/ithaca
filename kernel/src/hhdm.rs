use core::ptr::NonNull;

use limine::HhdmRequest;

use crate::types::PhysAddr;

static HHDM_REQUEST: HhdmRequest = HhdmRequest::new(0);

#[derive(Debug, Clone)]
pub struct Hhdm {
    base: u64,
}

impl Hhdm {
    pub fn with_limine() -> Hhdm {
        Hhdm {
            base: HHDM_REQUEST
                .get_response()
                .get()
                .expect("failed to retrieve higher half mapping")
                .offset,
        }
    }

    pub fn to_virtual<T>(&self, phys: PhysAddr) -> HigherHalf<T> {
        let addr = phys.0 + self.base;
        let ptr = unsafe { NonNull::new_unchecked(addr as usize as *mut T) };
        HigherHalf(ptr)
    }

    pub fn to_physical<T>(&self, addr: HigherHalf<T>) -> PhysAddr {
        PhysAddr(addr.as_ptr() as usize as u64 - self.base)
    }
}

/// A [NonNull] that points to memory in the higher half of the address space.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HigherHalf<T>(NonNull<T>);

impl<T> HigherHalf<T> {
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        self.0.as_ref()
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn as_nonnull(&self) -> NonNull<T> {
        self.0
    }
}

impl<T> Clone for HigherHalf<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for HigherHalf<T> {}
