pub mod x86_64;

pub unsafe fn init() {
    x86_64::init();
}

pub fn disable() {
    x86_64::disable();
}

pub unsafe fn enable() {
    x86_64::enable();
}

pub fn are_enabled() -> bool {
    x86_64::are_enabled()
}

pub fn wait() {
    x86_64::wait();
}

pub fn without<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    if are_enabled() {
        struct IrqGuard;

        impl Drop for IrqGuard {
            fn drop(&mut self) {
                unsafe { enable() };
            }
        }

        disable();
        let _guard = IrqGuard;
        f()
    } else {
        f()
    }
}
