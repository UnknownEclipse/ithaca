#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, allocator_api, step_trait, naked_functions)]

extern crate alloc;

use core::{
    arch::asm,
    fmt::{self, Write},
    iter::Step,
    panic::PanicInfo,
};

use owo_colors::{style, OwoColorize};
use serial_port::{SerialPort, SpinWriter};
use spin::{
    mutex::{SpinMutex, SpinMutexGuard},
    Lazy,
};

use crate::{
    address_space::{AddrSpace, MapOptions},
    types::Frame,
    x86_64::{
        apic::local::{LocalApic, X2Apic, XApic},
        pic,
    },
};

mod address_space;
mod boot;
mod dbg;
mod hhdm;
mod interrupts;
mod kernel_alloc;
mod pmm;
mod serial_port;
mod spinlock;
mod thread;
mod types;
mod vmm;
mod x86_64;

static COM1: Lazy<SpinMutex<SpinWriter>> = Lazy::new(|| {
    let port = unsafe { SerialPort::com1() };
    let writer = SpinWriter::new(port);
    SpinMutex::new(writer)
});

fn kernel_main() {
    log::set_logger(&Logger).ok();
    log::set_max_level(log::LevelFilter::Debug);
    log::info!("Hello!");

    unsafe {
        interrupts::init();
        kernel_alloc::init().expect("failed to initialize global kernel allocator");
    }

    let map_options = MapOptions {
        writable: true,
        disable_cache: true,
        ..Default::default()
    };
    let addr = XApic::physical_address();
    let frame = Frame(addr);
    let frame_range_end = Step::forward(frame, 1);
    let local_apic_address = AddrSpace::kernel()
        .map_frames(frame..frame_range_end, map_options)
        .unwrap();

    unsafe {
        pic::init(40, 48);
        pic::write_masks([0xff, 0xff]);

        let xapic = XApic::with_address(local_apic_address.cast());
        let mut lapic = LocalApic::enable(xapic).unwrap();
        lapic.enable_timer();
        // pic::init(32, 40);
        // pic::write_masks([0xfe, 0xff]);
    }
    // let mut local_apic = unsafe {
    //     LocalApicBuilder::with_addresses(addr, ptr.cast())
    //         .finish()
    //         .unwrap()
    // };

    // local_apic.enable_timer();
    // log::info!("{:#x?}", local_apic);

    loop {
        unsafe { interrupts::enable() };
        interrupts::wait();
    }
    log::info!("kernel exit");
}

#[derive(Debug)]
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let level_style = style().bold();
        let level_style = match record.level() {
            log::Level::Error => level_style.red(),
            log::Level::Warn => level_style.yellow(),
            log::Level::Info => level_style.green(),
            log::Level::Debug => level_style.blue(),
            log::Level::Trace => level_style.white(),
        };
        let mut writer = DbgWriter::lock();
        _ = writeln!(
            writer,
            "[{}][{}] {}",
            record.level().style(level_style),
            record.target().bold(),
            record.args()
        );
    }

    fn flush(&self) {}
}

#[no_mangle]
unsafe extern "C" fn _start() -> ! {
    asm!("xor rbp, rbp");

    kernel_main();

    // // Ensure we got a framebuffer.
    // if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response().get() {
    //     if framebuffer_response.framebuffer_count < 1 {
    //         hcf();
    //     }

    //     // Get the first framebuffer's information.
    //     let framebuffer = &framebuffer_response.framebuffers()[0];

    //     for i in 0..100_usize {
    //         // Calculate the pixel offset using the framebuffer information we obtained above.
    //         // We skip `i` scanlines (pitch is provided in bytes) and add `i * 4` to skip `i` pixels forward.
    //         let pixel_offset = i * framebuffer.pitch as usize + i * 4;

    //         // Write 0xFFFFFFFF to the provided pixel offset to fill it white.
    //         // We can safely unwrap the result of `as_ptr()` because the framebuffer address is
    //         // guaranteed to be provided by the bootloader.
    //         unsafe {
    //             *(framebuffer
    //                 .address
    //                 .as_ptr()
    //                 .unwrap()
    //                 .offset(pixel_offset as isize) as *mut u32) = 0xffffffff;
    //         }
    //     }
    // }

    hcf();
}

#[panic_handler]
fn rust_panic(info: &PanicInfo) -> ! {
    interrupts::disable();

    let mut writer = DbgWriter::lock();

    _ = writeln!(writer, "{}", "KERNEL PANIC".bold().red());
    _ = writeln!(writer, "{}", info);

    hcf();
}
struct DbgWriter {
    com1: SpinMutexGuard<'static, SpinWriter>,
}

impl DbgWriter {
    fn lock() -> DbgWriter {
        DbgWriter { com1: COM1.lock() }
    }
}

impl fmt::Write for DbgWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.com1.write_str(s)
    }
}

fn hcf() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}
