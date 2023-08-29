use core::ptr::NonNull;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

pub struct Hpet {
    base: NonNull<u64>,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
struct GeneralCapabilities {
    counter_clock_period: u32,
    vendor_id: u16,
    flags: u8,
    revision_id: u8,
}

impl GeneralCapabilities {
    pub fn timer_count(&self) -> u8 {
        self.flags + 1
    }
}

bitflags! {
    #[repr(C, align(8))]
    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    struct GeneralConfiguration: u64 {
        const ENABLE = 1;
    }
}
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
struct GeneralInterruptStatus {
    _reserved: u32,
    timer_interrupt_active_bitset: u32,
}
