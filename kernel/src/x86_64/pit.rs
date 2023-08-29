use super::{in16, out16};

pub struct Pit(());

impl Pit {
    pub fn current_count(&self) -> u16 {
        unsafe { in16(0x40) }
    }

    pub fn set_reload_value(&mut self, value: u16) {
        unsafe { out16(0x40, value) };
    }

    pub fn write_command(
        &mut self,
        channel: Channel,
        access_mode: AccessMode,
        operating_mode: OperatingMode,
    ) {
    }
}
pub fn sleep() {}

#[derive(Debug, Clone, Copy)]
enum Channel {
    Channel0,
    Channel1,
    Channel2,
}
#[derive(Debug, Clone, Copy)]
enum AccessMode {
    LatchCountValue,
    LowByteOnly,
    HighByteOnly,
    LowHighByte,
}
#[derive(Debug, Clone, Copy)]
enum OperatingMode {
    IrqOnTerminalCount,
    HardwareRetriggerableOneShot,
    RateGenerator,
    SquareWaveGenerator,
    SoftwareTriggeredStrobe,
    HardwareTriggeredStrobe,
}
