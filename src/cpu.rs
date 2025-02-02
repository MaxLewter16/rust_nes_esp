// Primary Registers?
const STACK_RESET: u8 = 0xff;
const STACK: u16 = 0x0100;

struct Program {
    file: Vec<u8>,
}

impl Program {
    fn get(&self, location: u16) -> u8 {
        self.file[location as usize]
    }
}

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

#[repr(u8)]
pub enum AddressingMode {
    XIndexIndirect = 0,
    Immediate = 0b00001000,
}

#[repr(u8)]
pub enum OpGroup {
    Zero = 0,
    One = 0x1,
    Two = 0x2,
}

#[repr(u8)]
pub enum InstrOp {
    Or = 0,
}

struct Opcode {
    instr: InstrOp,
    mode: AddressingMode,
    group: OpGroup
}

impl From<u8> for Opcode {
    // should attempt to parse byte into valid opcode, return None if byte is invalid
    fn from(value: u8) -> Option<Self> {

    }
}

const BRK: Opcode = Opcode{instr: InstrOp::Or, mode: AddressingMode::XIndexIndirect, group: OpGroup::Zero};

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
    pub program: Program,
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
            program: Program::from_file(""),
            program_counter: 0,
            idx_register_x: 0,
            idx_register_y: 0,
            processor_status: ProcessorStatus::new(),
            stack_pointer: STACK_RESET,
            accumulator: 0
        }
    }

    fn advance(&mut self) {
        let current = self.program.current();
        match(Opcode::from(current)) {
            //some instructions are unique and we match the whole opcode
            BRK => self.breaki(),
            // some groups of instructions have similar behavior we can unify
            Opcode{instr, mode: AddressingMode::Immediate, group: OpGroup::Two} => {
                let imm = self.get_immediate();
                match(instr) {
                    InstrOp::Or => self.or(imm),
                    _ => unimplemented!(),
                }
            }
            _ => unimplemented!(),
        }
    }

    // named breaki because 'break' is a keyword in rust
    fn breaki(&mut self) {

    }

    fn get_immediate(&mut self) {

    }

    fn or(&mut self, data: u8) {};

}