use core::iter::Step;

use bytemuck::{NoUninit, Zeroable};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, NoUninit)]
pub struct PhysAddr(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame(pub PhysAddr);

impl Step for Frame {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        end.0
             .0
            .checked_sub(start.0 .0)
            .map(|v| v / 4096)
            .and_then(|v| v.try_into().ok())
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        count
            .checked_mul(4096)
            .and_then(|offset| start.0 .0.checked_add(offset as u64))
            .map(|addr| Self(PhysAddr(addr)))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        count
            .checked_mul(4096)
            .and_then(|offset| start.0 .0.checked_sub(offset as u64))
            .map(|addr| Self(PhysAddr(addr)))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, NoUninit)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn addr(&self) -> usize {
        self.0
    }

    pub fn as_ptr(&self) -> *mut () {
        self.0 as *mut ()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, NoUninit)]
pub struct Page(pub VirtAddr);

impl Step for Page {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        end.0.addr().checked_sub(start.0.addr()).map(|v| v / 4096)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        count
            .checked_mul(4096)
            .and_then(|offset| start.0.addr().checked_add(offset))
            .map(|addr| Self(VirtAddr(addr)))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        count
            .checked_mul(4096)
            .and_then(|offset| start.0.addr().checked_sub(offset))
            .map(|addr| Self(VirtAddr(addr)))
    }
}
