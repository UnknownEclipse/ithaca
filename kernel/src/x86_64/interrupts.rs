use super::pic;

#[derive(Debug)]
pub enum InterruptController {
    Pic,
}

impl InterruptController {
    pub unsafe fn end_of_interrupt(&self, vector: u8) {
        match self {
            InterruptController::Pic => pic::end_of_interrupt(vector, PIC1_OFFSET, PIC2_OFFSET),
        }
    }
}

const PIC1_OFFSET: u8 = 32;
const PIC2_OFFSET: u8 = 40;
