use core::arch::asm;

use bitflags::bitflags;

pub mod apic;
pub mod hpet;
pub mod idt;
pub mod interrupts;
pub mod local_apic;
pub mod pic;
pub mod pit;
pub mod segment;

#[inline]
pub unsafe fn out8(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value);
}

#[inline]
pub unsafe fn out16(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value);
}

#[inline]
pub unsafe fn in8(port: u16) -> u8 {
    let value;
    asm!("in al, dx", in("dx") port, out("al") value);
    value
}

#[inline]
pub unsafe fn in16(port: u16) -> u16 {
    let value;
    asm!("in ax, dx", in("dx") port, out("ax") value);
    value
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct RFlags: u64 {
        const INTERRUPTS_ENABLED = 1 << 9;
    }
}

impl RFlags {
    pub fn read() -> Self {
        let bits;
        unsafe {
            asm!("pushfq", "pop {}", out(reg) bits, options(nomem, nostack, preserves_flags))
        };
        RFlags::from_bits_retain(bits)
    }
}

pub mod cr3 {
    use core::arch::asm;

    use crate::types::{Frame, PhysAddr};

    pub fn read() -> Frame {
        let bits: u64;
        unsafe { asm!("mov {}, cr3", out(reg) bits, options(nomem, nostack, preserves_flags)) };
        Frame(PhysAddr(bits & !0xfff))
    }
}

pub mod cr2 {
    use core::arch::asm;

    use crate::types::VirtAddr;

    pub fn read() -> VirtAddr {
        let bits: usize;
        unsafe { asm!("mov {}, cr2", out(reg) bits, options(nomem, nostack, preserves_flags)) };
        VirtAddr(bits)
    }
}

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let high: u64;
    let low: u64;
    asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high);
    (high << 32) | low
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = value.wrapping_shr(32) as u32;

    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high);
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskState {
    pub rbx: usize,
    pub rbp: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
}

pub unsafe extern "C" fn context_switch(from: *mut *const TaskState, to: *const TaskState) {
    asm!(
        "push rbx",
        "push rbp",
        "push rdi",
        "push rsi",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov [{from}], rsp",
        "mov rsp, {to}",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rsi",
        "pop rdi",
        "pop rbp",
        "pop rbx",
        from = in(reg) from,
        to = in(reg) to,
    );
}
