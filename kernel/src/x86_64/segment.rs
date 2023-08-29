#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selector(pub u16);

pub mod code {
    use core::arch::asm;

    use super::Selector;

    pub fn read() -> Selector {
        let value;
        unsafe { asm!("mov {:x}, cs", out(reg) value, options(nomem, nostack, preserves_flags)) };
        Selector(value)
    }
}
