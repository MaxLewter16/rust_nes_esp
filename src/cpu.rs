// Primary Registers?
const STACK_RESET: u8 = 0xff;
const STACK: u16 = 0x0100;
pub struct CPU {
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub accumulator: u8,
    pub idx_register_x: u8,
    pub idx_register_y: u8,
        // Maybe use bitflags! ? 
    ///  7 6 5 4 3 2 1 0
    ///  N V _ B D I Z C
    ///  | |   | | | | +--- Carry Flag
    ///  | |   | | | +----- Zero Flag
    ///  | |   | | +------- Interrupt Disable
    ///  | |   | +--------- Decimal Mode (not used on NES)
    ///  | |   +----------- Break Command
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    pub processor_status: u8,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            program_counter: 0,
            idx_register_x: 0,
            idx_register_y: 0,
            processor_status: 0,
            stack_pointer: STACK_RESET,
            accumulator: 0
        }
    }
}