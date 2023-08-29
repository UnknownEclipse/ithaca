use core::{arch::x86_64::__cpuid, fmt::Debug, ptr::NonNull};

use bitflags::bitflags;

use crate::{
    types::PhysAddr,
    x86_64::{rdmsr, wrmsr},
};

#[derive(Debug)]
pub enum LocalApicInitError {
    NotSupported,
}

#[derive(Debug)]
pub struct LocalApicBuilder {
    addr: NonNull<PaddedRegister>,
    spurious_interrupt_vector: u8,
    physical: PhysAddr,
}

impl LocalApicBuilder {
    pub fn with_addresses(physical: PhysAddr, ptr: NonNull<()>) -> Self {
        Self {
            addr: ptr.cast(),
            physical,
            spurious_interrupt_vector: 0xff,
        }
    }

    pub unsafe fn finish(self) -> Result<LocalApic, LocalApicInitError> {
        if !is_apic_present() {
            return Err(LocalApicInitError::NotSupported);
        }

        let mut lapic = LocalApic {
            phys: self.physical,
            ptr: self.addr,
        };

        lapic.msr_enable();
        lapic.set_spurious_interrupt_vector(self.spurious_interrupt_vector);
        lapic.software_enable();

        Ok(lapic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalApicId(u32);

pub struct LocalApic {
    phys: PhysAddr,
    ptr: NonNull<PaddedRegister>,
}

impl LocalApic {
    pub fn id(&self) -> LocalApicId {
        LocalApicId(unsafe {
            self.read_register(RegisterIndex::LocalApicId)
                .wrapping_shr(24)
        })
    }

    pub fn version(&self) -> u32 {
        unsafe { self.read_register(RegisterIndex::LocalApicVersion) & 0xff }
    }

    pub unsafe fn end_of_interrupt(&mut self) {
        self.write_register(RegisterIndex::Eoi, 0);
    }

    pub unsafe fn set_spurious_interrupt_vector(&mut self, vector: u8) {
        let old = self.read_register(RegisterIndex::SpuriousInterruptVector);
        let new = (old & !0xff) | u32::from(vector);
        self.write_register(RegisterIndex::SpuriousInterruptVector, new);
    }

    pub fn enable_timer(&mut self) {
        const PERIODIC: u32 = 0x20000;

        unsafe {
            // let timer_register = LocalVectorTableEntry::new(32, LvtEntryFlags::empty());
            self.write_register(RegisterIndex::Timer, 32 | PERIODIC);
            self.write_register(RegisterIndex::TimerDivider, 0x3);
            self.write_register(RegisterIndex::TimerCountInitial, 0x10);
        }
    }

    unsafe fn msr_enable(&mut self) {
        let msr_value = (self.phys.0 & !0xfff) | APIC_ENABLE;
        wrmsr(IA32_APIC_BASE, msr_value);
    }

    unsafe fn write_register(&self, register: RegisterIndex, value: u32) {
        self.ptr
            .as_ptr()
            .add(register as usize)
            .write_volatile(PaddedRegister(value));
    }

    fn is_bootstrap_processor(&self) -> bool {
        unsafe { rdmsr(IA32_APIC_BASE) & (1 << 8) != 0 }
    }

    unsafe fn read_register(&self, register: RegisterIndex) -> u32 {
        self.ptr.as_ptr().add(register as usize).read_volatile().0
    }

    unsafe fn software_enable(&mut self) {
        let r = RegisterIndex::SpuriousInterruptVector;
        self.write_register(r, self.read_register(r) | 0x100);
    }

    unsafe fn software_disable(&mut self) {
        let r = RegisterIndex::SpuriousInterruptVector;
        self.write_register(r, self.read_register(r) & !0x100);
    }
}

impl Debug for LocalApic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LocalApic")
            .field("address", &self.phys)
            .field("id", &self.id())
            .field("version", &self.version())
            .finish()
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct PaddedRegister(u32);

fn is_apic_present() -> bool {
    let result = unsafe { __cpuid(1) };
    result.edx & (1 << 9) != 0
}

const IA32_APIC_BASE: u32 = 0x1b;
const APIC_ENABLE: u64 = 1 << 11;

#[derive(Debug, Clone, Copy)]
enum RegisterIndex {
    LocalApicId = 2,
    LocalApicVersion = 3,
    Eoi = 0xb0 / 0x10,
    SpuriousInterruptVector = 15,
    Timer = 0x320 / 0x10,
    TimerCountInitial = 0x380 / 0x10,
    TimerCountCurrent = 0x390 / 0x10,
    TimerDivider = 0x3e0 / 0x10,
}

pub fn read_apic_base() -> PhysAddr {
    let bits = unsafe { rdmsr(IA32_APIC_BASE) } & 0xffffff000;
    PhysAddr(bits)
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct LvtEntry(u32);

impl LvtEntry {
    pub fn new(vector: u8, flags: LvtEntryFlags) -> Self {
        let flags = flags & LvtEntryFlags::all();
        Self(u32::from(vector) | flags.bits())
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct LvtEntryFlags: u32 {
        const PENDING = 1 << 12;
        const LOW_TRIGGERED = 1 << 13;
        const REMOTE_IRR = 1 << 14;
        const LEVEL_TRIGGERED = 1 << 15;
        const MASKED = 1 << 16;
    }
}
