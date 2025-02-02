// Primary Registers?
const STACK_RESET: u8 = 0xff;
const STACK: u16 = 0x0100;
#[repr(u8)]
pub enum ProcessorStatusFlag{
    ///  7 6 5 4 3 2 1 0
    ///  N V _ B D I Z C
    ///  | |   | | | | +--- Carry Flag
    ///  | |   | | | +----- Zero Flag
    ///  | |   | | +------- Interrupt Disable
    ///  | |   | +--------- Decimal Mode (not used on NES)
    ///  | |   +----------- Break Command
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    Carry = 1, 
    Zero = 1 << 1,
    Interrupt = 1 << 2,
    Decimal = 1 << 3,
    Break = 1 << 4,
    Overflow = 1 << 6,
    Negative = 1 << 7
}

pub struct ProcessorStatus{
    pub flags: u8
}

impl ProcessorStatus {
    pub fn new() -> Self {
        Self { flags: 0 }
    }
    pub fn set(&mut self, flag: ProcessorStatusFlag) {
        self.flags |= flag as u8;
    }
    pub fn clear(&mut self, flag: ProcessorStatusFlag) {
        self.flags &= !(flag as u8);
    }
}

pub struct CPU {
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub accumulator: u8,
    pub idx_register_x: u8,
    pub idx_register_y: u8,
    pub processor_status: ProcessorStatus,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            program_counter: 0,
            idx_register_x: 0,
            idx_register_y: 0,
            processor_status: ProcessorStatus::new(),
            stack_pointer: STACK_RESET,
            accumulator: 0
        }
    }
}