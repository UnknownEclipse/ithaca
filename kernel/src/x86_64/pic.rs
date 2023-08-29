use crate::x86_64::{in8, out8};

pub unsafe fn init(pic1_offset: u8, pic2_offset: u8) {
    let masks = read_masks();

    out8(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_pause();
    out8(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_pause();
    out8(PIC1_DATA, pic1_offset);
    io_pause();
    out8(PIC2_DATA, pic2_offset);
    io_pause();
    out8(PIC1_DATA, 4);
    io_pause();
    out8(PIC2_DATA, 2);
    io_pause();

    out8(PIC1_DATA, ICW4_8086);
    io_pause();
    out8(PIC2_DATA, ICW4_8086);
    io_pause();

    write_masks(masks);
}

pub unsafe fn end_of_interrupt(vector: u8, pic1_offset: u8, pic2_offset: u8) {
    if (pic1_offset..pic1_offset + 8).contains(&vector) {
        out8(PIC1_COMMAND, PIC_EOI);
    } else if (pic2_offset..pic2_offset + 8).contains(&vector) {
        out8(PIC2_COMMAND, PIC_EOI);
    } else {
        panic!("invalid irq vector for pic");
    }
}

pub fn read_masks() -> [u8; 2] {
    unsafe { [in8(PIC1_DATA), in8(PIC2_DATA)] }
}

pub unsafe fn write_masks(masks: [u8; 2]) {
    out8(PIC1_DATA, masks[0]);
    out8(PIC2_DATA, masks[1]);
}

const PIC1_COMMAND: u16 = 0x0020;
const PIC1_DATA: u16 = 0x0021;
const PIC2_COMMAND: u16 = 0x00a0;
const PIC2_DATA: u16 = 0x00a1;
const ICW1_ICW4: u8 = 0x01; /* Indicates that ICW4 will be present */
const ICW1_SINGLE: u8 = 0x02; /* Single (cascade) mode */
const ICW1_INTERVAL4: u8 = 0x04; /* Call address interval 4 (8) */
const ICW1_LEVEL: u8 = 0x08; /* Level triggered (edge) mode */
const ICW1_INIT: u8 = 0x10; /* Initialization - required! */
const ICW4_8086: u8 = 0x01; /* 8086/88 (MCS-80/85) mode */
const ICW4_AUTO: u8 = 0x02; /* Auto (normal) EOI */
const ICW4_BUF_SLAVE: u8 = 0x08; /* Buffered mode/slave */
const ICW4_BUF_MASTER: u8 = 0x0c; /* Buffered mode/master */
const ICW4_SFNM: u8 = 0x10; /* Special fully nested (not) */
const PIC_EOI: u8 = 0x20;

#[inline]
unsafe fn io_pause() {
    out8(0x80, 0);
}
