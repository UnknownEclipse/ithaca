use core::{arch::x86_64::__cpuid, ptr::NonNull};

use bitflags::bitflags;

use crate::{
    hhdm::Hhdm,
    types::{PhysAddr, VirtAddr},
    x86_64::{rdmsr, wrmsr},
};

#[derive(Debug)]
pub enum LocalApicP {
    XApic(LocalApic<XApic>),
    X2Apic(LocalApic<X2Apic>),
}

#[derive(Debug)]
pub struct UnsupportedError;

#[derive(Debug)]
pub enum ApicEnableError {
    Unsupported,
}

#[derive(Debug)]
pub struct LocalApic<A> {
    address_space: A,
}

impl<A> LocalApic<A>
where
    A: ApicAddressSpace,
{
    pub unsafe fn enable(address_space: A) -> Result<Self, ApicEnableError> {
        let spurious_interrupt_vector = 0xff;
        let spurious_interrupt_vector_register = 0xf;

        address_space.enable()?;

        address_space.write(
            spurious_interrupt_vector_register,
            address_space.read(spurious_interrupt_vector_register) | 0x1ff,
        );

        Ok(Self { address_space })
    }

    pub fn id(&self) -> LocalApicId {
        unsafe { self.address_space.id() }
    }

    pub fn version(&self) -> u8 {
        unsafe { self.address_space.read(0x3) as u8 }
    }

    pub fn enable_timer(&mut self) {
        let entry_bits = pack_timer_lvt_entry(32, TimerMode::Periodic, TriggerMode::Edge, false);
        unsafe {
            self.address_space.write(0x38, 0x1);
            self.address_space.write(0x3e, 0x3);
            self.address_space.write(0x32, entry_bits);
        };
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct TimerLvtFlags: u32 {

        const INTERRUPT_PENDING = 1 << 12;
    }
}

fn pack_timer_lvt_entry(
    vector: u8,
    timer_mode: TimerMode,
    trigger_mode: TriggerMode,
    mask: bool,
) -> u32 {
    u32::from(vector)
        | ((timer_mode as u32) << 17)
        | (u32::from(mask) << 16)
        | ((trigger_mode as u32) << 15)
}

#[derive(Debug, Clone, Copy)]
enum TriggerMode {
    Edge,
    Level,
}

#[derive(Debug, Clone, Copy)]
enum TimerMode {
    OneShot,
    Periodic,
    TscDeadline,
}

#[derive(Debug, Clone, Copy)]
pub struct LocalApicId(u32);

pub unsafe trait ApicAddressSpace {
    unsafe fn id(&self) -> LocalApicId;
    unsafe fn enable(&self) -> Result<(), ApicEnableError>;
    unsafe fn read(&self, register_index: u32) -> u32;
    unsafe fn write(&self, register_index: u32, value: u32);
}

#[derive(Debug)]
pub struct XApic {
    base: NonNull<Register>,
}

impl XApic {
    pub fn physical_address() -> PhysAddr {
        XAPIC_BASE_ADDRESS
    }

    pub unsafe fn with_address(addr: NonNull<()>) -> Self {
        Self { base: addr.cast() }
    }

    pub fn with_higher_half() -> Self {
        let hhdm = Hhdm::with_limine();
        let base = hhdm.to_virtual(XAPIC_BASE_ADDRESS).as_nonnull();
        Self { base }
    }
}

unsafe impl ApicAddressSpace for XApic {
    unsafe fn id(&self) -> LocalApicId {
        let bits = unsafe { self.read(0x2) };
        LocalApicId(bits.wrapping_shr(24))
    }

    unsafe fn enable(&self) -> Result<(), ApicEnableError> {
        let supported = unsafe { __cpuid(1).edx & (1 << 9) != 0 };
        if !supported {
            return Err(ApicEnableError::Unsupported);
        }

        let value = rdmsr(IA32_APIC_BASE);
        let new = value | (1 << 11);
        wrmsr(IA32_APIC_BASE, new);
        Ok(())
    }

    unsafe fn read(&self, register_index: u32) -> u32 {
        self.base
            .as_ptr()
            .add(register_index as usize)
            .read_volatile()
            .0
    }

    unsafe fn write(&self, register_index: u32, value: u32) {
        self.base
            .as_ptr()
            .add(register_index as usize)
            .write_volatile(Register(value));
    }
}

#[derive(Debug)]
pub struct X2Apic;

unsafe impl ApicAddressSpace for X2Apic {
    unsafe fn id(&self) -> LocalApicId {
        let bits = unsafe { self.read(0x2) };
        LocalApicId(bits)
    }

    unsafe fn enable(&self) -> Result<(), ApicEnableError> {
        let supported = unsafe { __cpuid(1).ecx & (1 << 21) != 0 };
        if !supported {
            return Err(ApicEnableError::Unsupported);
        }

        wrmsr(IA32_APIC_BASE, XAPIC_BASE_ADDRESS.0 | (0b11 << 10));
        Ok(())
    }

    unsafe fn read(&self, register_index: u32) -> u32 {
        rdmsr(X2APIC_MSR_BASE + register_index) as u32
    }

    unsafe fn write(&self, register_index: u32, value: u32) {
        wrmsr(X2APIC_MSR_BASE + register_index, value.into());
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct Register(u32);

const X2APIC_MSR_BASE: u32 = 0x800;
const IA32_APIC_BASE: u32 = 0x1b;
const XAPIC_BASE_ADDRESS: PhysAddr = PhysAddr(0xfee00000);

// #[repr(C, align(4096))]
// struct Registers {
//     _reserved1: [u32; 2],
//     id: Register<u32>,
//     version: Register<u32>,
//     _reserved2: [u32; 4],
//     task_priority: Register<u32>,
//     arbitration_priority: Register<u32>,
//     processor_priority: Register<u32>,
//     end_of_interrupt: Register<u32>,
//     remote_read: Register<u32>,
//     logical_destination: Register<u32>,
//     destination_format: Register<u32>,
//     spurious_interrupt_vector: Register<u32>,
//     in_service: [Register<u32>; 8],
//     trigger_mode: [Register<u32>; 8],
//     interrupt_request: [Register<u32>; 8],
//     error_status: Register<u32>,
//     _reserved3: [Register<u32>; 6],
//     corrected_machine_check_interrupt: Register<u32>,
//     interrupt_command: [Register<u32>; 2],
//     timer: Register<u32>,
//     thermal_sensor: Register<u32>,
//     performance_monitoring_counters: Register<u32>,
//     lint0: Register<u32>,
//     lint1: Register<u32>,
//     error: Register<u32>,
//     timer_count_initial: Register<u32>,
//     timer_count_current: Register<u32>,
//     _reserved4: [Register<u32>; 4],
//     timer_divider: Register<u32>,
//     _reserved5: Register<u32>,
// }

// #[repr(C, align(16))]
// struct Register<T>(Cell<T>);

// impl<T> Register<T>
// where
//     T: Copy,
// {
//     pub unsafe fn write(&self, value: T) {
//         unsafe { self.0.as_ptr().write_volatile(value) };
//     }

//     pub unsafe fn read(&self) -> T {
//         unsafe { self.0.as_ptr().read_volatile() }
//     }
// }
