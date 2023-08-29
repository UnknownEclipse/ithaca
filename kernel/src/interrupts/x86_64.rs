use core::arch::asm;

use spin::Lazy;

use crate::x86_64::{
    cr2,
    idt::{Idt, RawGate},
    RFlags,
};

pub enum InterruptController {}

pub unsafe fn init() {
    IDT.load();
}

pub unsafe fn enable() {
    unsafe { asm!("sti", options(nomem, nostack)) };
}

pub fn disable() {
    unsafe { asm!("cli", options(nomem, nostack)) };
}

pub fn wait() {
    unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) };
}

pub unsafe fn enable_and_wait() {
    unsafe { asm!("sti; hlt", options(nomem, nostack)) };
}

pub fn disable_and_wait() {
    unsafe { asm!("cli; hlt", options(nomem, nostack)) };
}

pub fn are_enabled() -> bool {
    RFlags::read().contains(RFlags::INTERRUPTS_ENABLED)
}

static IDT: Lazy<Idt> = Lazy::new(build_idt);

#[repr(C)]
#[derive(Debug)]
pub struct StackFrame {
    pub ip: usize,
    pub cs: usize,
    pub flags: usize,
    pub sp: usize,
    pub ss: usize,
}

fn build_idt() -> Idt {
    let mut idt = Idt {
        divide_error: RawGate::with_addr(divide_error_handler as usize),
        debug: RawGate::with_addr(debug_handler as usize),
        non_maskable_interrupt: RawGate::with_addr(nmi_handler as usize),
        breakpoint: RawGate::with_addr(breakpoint_handler as usize),
        overflow: RawGate::with_addr(overflow_handler as usize),
        bound_range_exceeded: RawGate::with_addr(bound_range_exceeded_handler as usize),
        invalid_opcode: RawGate::with_addr(invalid_opcode_handler as usize),
        device_not_available: RawGate::with_addr(device_not_available_handler as usize),
        double_fault: RawGate::with_addr(double_fault_handler as usize),
        invalid_tss: RawGate::with_addr(invalid_tss_handler as usize),
        segment_not_present: RawGate::with_addr(segment_not_present_handler as usize),
        stack_segment_fault: RawGate::with_addr(stack_segment_fault_handler as usize),
        general_protection_fault: RawGate::with_addr(general_protection_fault_handler as usize),
        page_fault: RawGate::with_addr(page_fault_handler as usize),
        x87_floating_point: RawGate::with_addr(x87_floating_point_handler as usize),
        alignment_check: RawGate::with_addr(alignment_check_handler as usize),
        machine_check: RawGate::with_addr(machine_check_handler as usize),
        simd_floating_point: RawGate::with_addr(simd_floating_point_handler as usize),
        virtualization_exception: RawGate::with_addr(virtualization_exception_handler as usize),
        control_protection_exception: RawGate::with_addr(
            control_protection_exception_handler as usize,
        ),
        hypervisor_injection: RawGate::with_addr(hypervisor_injection_handler as usize),
        vmm_communication: RawGate::with_addr(vmm_communication_handler as usize),
        security: RawGate::with_addr(security_handler as usize),

        ..Idt::empty()
    };
    idt.gates[0].set_addr(timer_handler as usize);

    idt
}

extern "x86-interrupt" fn divide_error_handler(_frame: StackFrame) {
    todo!("divide error handling");
}
extern "x86-interrupt" fn debug_handler(_frame: StackFrame) {
    todo!("debug handling");
}
extern "x86-interrupt" fn nmi_handler(_frame: StackFrame) {
    todo!("non-maskable interrupt handling");
}
extern "x86-interrupt" fn breakpoint_handler(_frame: StackFrame) {
    log::info!("BREAKPOINT");
}
extern "x86-interrupt" fn overflow_handler(_frame: StackFrame) {
    todo!("overflow handling");
}
extern "x86-interrupt" fn bound_range_exceeded_handler(_frame: StackFrame) {
    todo!("bound range exceeded handling");
}
extern "x86-interrupt" fn invalid_opcode_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn device_not_available_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn double_fault_handler(_frame: StackFrame, _error: u64) -> ! {
    panic!("DOUBLE FAULT");
}
extern "x86-interrupt" fn invalid_tss_handler(_frame: StackFrame, _error: u64) {
    todo!()
}
extern "x86-interrupt" fn segment_not_present_handler(_frame: StackFrame, _error: u64) {
    todo!()
}
extern "x86-interrupt" fn stack_segment_fault_handler(_frame: StackFrame, _error: u64) {
    todo!()
}
extern "x86-interrupt" fn general_protection_fault_handler(_frame: StackFrame, error: u64) -> ! {
    panic!("GENERAL PROTECTION FAULT: {:#b}", error);
}
extern "x86-interrupt" fn page_fault_handler(_frame: StackFrame, error: u64) -> ! {
    panic!("PAGE FAULT: {:#x?}: {:#b}", cr2::read(), error);
}
extern "x86-interrupt" fn x87_floating_point_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn alignment_check_handler(_frame: StackFrame, _error: u64) {
    todo!()
}
extern "x86-interrupt" fn machine_check_handler(_frame: StackFrame) -> ! {
    todo!()
}
extern "x86-interrupt" fn simd_floating_point_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn virtualization_exception_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn control_protection_exception_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn hypervisor_injection_handler(_frame: StackFrame) {
    todo!()
}
extern "x86-interrupt" fn vmm_communication_handler(_frame: StackFrame, _error: u64) {
    todo!()
}
extern "x86-interrupt" fn security_handler(_frame: StackFrame, _error: u64) {
    todo!()
}
extern "x86-interrupt" fn timer_handler(_frame: StackFrame) {
    log::info!("Timer!");
}
