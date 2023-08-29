use limine::{BootInfoRequest, HhdmRequest, KernelAddressRequest, MemmapRequest};

pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new(0);
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new(0);
pub static BOOTINFO_REQUEST: BootInfoRequest = BootInfoRequest::new(0);
pub static KERNEL_ADDRESS_REQUEST: KernelAddressRequest = KernelAddressRequest::new(0);
