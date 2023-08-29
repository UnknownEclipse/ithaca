use core::{arch::asm, ffi::c_void, marker::PhantomData, ops::ControlFlow};

use unwinding::abi::{UnwindContext, UnwindReasonCode, _Unwind_Backtrace, _Unwind_GetIP};

#[derive(Debug)]
pub struct Frame<'a> {
    ip: usize,
    _p: PhantomData<&'a ()>,
}

pub fn trace<F>(mut f: F)
where
    F: FnMut(Frame<'_>) -> ControlFlow<()>,
{
    let arg: *mut F = &mut f;
    _Unwind_Backtrace(trace_fn::<F>, arg.cast());
}

extern "C" fn trace_fn<F>(ctx: &UnwindContext<'_>, arg: *mut c_void) -> UnwindReasonCode
where
    F: FnMut(Frame<'_>) -> ControlFlow<()>,
{
    let f: *mut F = arg.cast();
    let f = unsafe { &mut *f };
    let ip = _Unwind_GetIP(ctx);

    let frame = Frame {
        ip,
        _p: PhantomData,
    };

    if f(frame).is_break() {
        UnwindReasonCode::NORMAL_STOP
    } else {
        UnwindReasonCode::CONTINUE_UNWIND
    }
}

fn read_rbp() -> usize {
    let value;
    unsafe { asm!("mov {}, rbp", out(reg) value) };
    value
}
