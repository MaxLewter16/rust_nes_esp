use std::ops::Index;

use crate::opmap::OP_MAP;

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
    pub memory: Program,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub accumulator: u8,
    pub idx_register_x: u8,
    pub idx_register_y: u8,
    pub processor_status: ProcessorStatus,
}

struct Memory {
    program: Program,
    mmio: MMIO,
    ram: [u8; 0x2000],
}

impl Index<u16> for Memory {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        if address < 0x2000 { self.ram[address] }
        else if address < 0x4020 {self.mmio.send_write(address)}
        else if address < 0x6000 {
            //Expansion ROM
            &0u8
        }
        else if address < 0x8000 {
            //SRAM
            &0u8
        }
        else {
            &self.program.get(address)
        }
    }
}

impl Memory {
    fn from_file(path: String) -> Self {

    }

    fn mmio(&self, address: u16) {
        match address {
            0x2000 => //PPU control register 1
        }
    }
}


impl CPU {
    pub fn new() -> Self {
        CPU {
            memory: Memory::from_file(""),
            program_counter: 0,
            idx_register_x: 0,
            idx_register_y: 0,
            processor_status: ProcessorStatus::new(),
            stack_pointer: STACK_RESET,
            accumulator: 0
        }
    }

    fn advance(&mut self) {
        OP_MAP[self.memory[self.program_counter]](self);
    }

    // named breaki because 'break' is a keyword in rust
    fn breaki(&mut self) {
        //execute break instruction
    }

    fn get_immediate(&mut self) -> u8 {
        //read next byte in Program, advancing program counter
    }

    fn get_absolute(&mut self) -> u16 {
        let low = self.memory[self.program_counter] as u16;       // Fetch low byte
        let high = self.memory[self.program_counter + 1] as u16;  // Fetch high byte
        self.program_counter += 2; 

        (high << 8) | low  // Combine into 16-bit address (little-endian)
    }

    pub fn or_immediate(&mut self) {
        self.get_immediate();
        //do arithmetic 'or'
        self.do_or();
    }

    pub fn or_absolute(&mut self) {
        self.get_absolute();
        self.do_or();
    }

    #[inline]
    pub fn do_or(&mut self) {

    }

    pub fn noop(&mut self) {}
}


