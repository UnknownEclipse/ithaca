use core::{arch::asm, mem};

use bitfrob::{u16_with_bit, u16_with_value};

use crate::x86_64::segment::{self, Selector};

#[repr(C, align(16))]
#[derive(Debug)]
pub struct Idt {
    pub divide_error: RawGate,
    pub debug: RawGate,
    pub non_maskable_interrupt: RawGate,
    pub breakpoint: RawGate,
    pub overflow: RawGate,
    pub bound_range_exceeded: RawGate,
    pub invalid_opcode: RawGate,
    pub device_not_available: RawGate,
    pub double_fault: RawGate,
    pub _coprocessor_segment_overrun: RawGate,
    pub invalid_tss: RawGate,
    pub segment_not_present: RawGate,
    pub stack_segment_fault: RawGate,
    pub general_protection_fault: RawGate,
    pub page_fault: RawGate,
    pub _reserved: RawGate,
    pub x87_floating_point: RawGate,
    pub alignment_check: RawGate,
    pub machine_check: RawGate,
    pub simd_floating_point: RawGate,
    pub virtualization_exception: RawGate,
    pub control_protection_exception: RawGate,
    pub _reserved2: [RawGate; 6],
    pub hypervisor_injection: RawGate,
    pub vmm_communication: RawGate,
    pub security: RawGate,
    pub _reserved3: RawGate,
    pub gates: [RawGate; 224],
}

const _: () = assert!(mem::size_of::<Idt>() == 256 * 16);
const _: () = assert!(mem::align_of::<Idt>() == 16);

impl Idt {
    pub fn empty() -> Self {
        Self {
            gates: [RawGate::missing(); 224],
            divide_error: RawGate::missing(),
            debug: RawGate::missing(),
            non_maskable_interrupt: RawGate::missing(),
            breakpoint: RawGate::missing(),
            overflow: RawGate::missing(),
            bound_range_exceeded: RawGate::missing(),
            invalid_opcode: RawGate::missing(),
            device_not_available: RawGate::missing(),
            double_fault: RawGate::missing(),
            _coprocessor_segment_overrun: RawGate::missing(),
            invalid_tss: RawGate::missing(),
            segment_not_present: RawGate::missing(),
            stack_segment_fault: RawGate::missing(),
            general_protection_fault: RawGate::missing(),
            page_fault: RawGate::missing(),
            _reserved: RawGate::missing(),
            x87_floating_point: RawGate::missing(),
            alignment_check: RawGate::missing(),
            machine_check: RawGate::missing(),
            simd_floating_point: RawGate::missing(),
            virtualization_exception: RawGate::missing(),
            control_protection_exception: RawGate::missing(),
            _reserved2: [RawGate::missing(); 6],
            hypervisor_injection: RawGate::missing(),
            vmm_communication: RawGate::missing(),
            security: RawGate::missing(),
            _reserved3: RawGate::missing(),
        }
    }

    pub unsafe fn load(&self) {
        #[repr(C, packed(2))]
        #[derive(Debug)]
        struct IdtPtr {
            limit: u16,
            base: u64,
        }

        let idt_ptr = IdtPtr {
            base: self as *const Self as usize as u64,
            limit: mem::size_of::<Idt>() as u16 - 1,
        };

        unsafe { asm!("lidt [{}]", in(reg) &idt_ptr) };
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawGate {
    offset_low: u16,
    selector: segment::Selector,
    options: GateOptions,
    offset_mid: u16,
    offset_high: u32,
    _reserved: u32,
}

impl RawGate {
    pub fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: Selector(0),
            options: GateOptions::new(),
            offset_mid: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    pub fn with_addr(addr: usize) -> Self {
        let mut gate = Self::missing();
        gate.set_addr(addr);
        gate
    }

    pub fn set_addr(&mut self, addr: usize) {
        self.selector = segment::code::read();
        self.offset_low = addr as u16;
        self.offset_mid = addr.wrapping_shr(16) as u16;
        self.offset_high = addr.wrapping_shr(32) as u32;
        self.options.set_present(true);
    }
}

#[derive(Debug, Clone, Copy)]
struct GateOptions(u16);

impl GateOptions {
    fn new() -> Self {
        Self(0b1110_0000_0000)
    }

    fn set_present(&mut self, present: bool) -> &mut Self {
        self.0 = u16_with_bit(15, self.0, present);
        self
    }

    fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0 = u16_with_bit(8, self.0, !disable);
        self
    }

    fn set_privilege_level(&mut self, dpl: u16) -> &mut Self {
        self.0 = u16_with_value(13, 14, self.0, dpl);
        self
    }

    unsafe fn set_stack_index(&mut self, index: u16) -> &mut Self {
        self.0 = u16_with_value(0, 2, self.0, index + 1);
        self
    }
}
