use std::{ops::{Deref, DerefMut, Index}, process::id};

use crate::opmap::OP_MAP;

// Primary Registers?
const STACK_RESET: u8 = 0xff;
const STACK: u16 = 0x0100;

struct Program {
    file: Vec<u8>,
}

impl Program {
    fn get(&self, location: u16) -> u8 {
        self.file[(location - 0x8000) as usize]
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

impl Deref for ProcessorStatusFlag {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        // safe because of primitive representation
        // see: https://doc.rust-lang.org/reference/items/enumerations.html
        unsafe { &*(self as *const Self as *const u8) }
    }
}

impl DerefMut for ProcessorStatusFlag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // safe because of primitive representation
        // see: https://doc.rust-lang.org/reference/items/enumerations.html
        unsafe { &mut *(self as *mut Self as *mut u8) }
    }
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

impl Deref for ProcessorStatus {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        // safe because of primitive representation
        // see: https://doc.rust-lang.org/reference/items/enumerations.html
        unsafe { &*(self as *const Self as *const u8) }
    }
}

impl DerefMut for ProcessorStatus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // safe because of primitive representation
        // see: https://doc.rust-lang.org/reference/items/enumerations.html
        unsafe { &mut *(self as *mut Self as *mut u8) }
    }
}

pub struct CPU {
    pub memory: Memory,
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

enum Register {
    X,
    Y
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
        let i = OP_MAP[self.memory[self.program_counter] as usize];
        self.program_counter += 1;
        i(self);
    }

    // named breaki because 'break' is a keyword in rust
    fn breaki(&mut self) {
        //execute break instruction
    }

    fn get_immediate(&mut self) -> u8 {
        let tmp = self.memory[self.program_counter];
        self.program_counter += 1;
        tmp
    }

    fn get_zero_page(&mut self) -> u8 {
        // assume upper address byte is 0
        let tmp = self.memory[self.memory[self.program_counter] as u16];
        self.program_counter += 1;
        tmp
    }

    fn get_zero_page_xy(&mut self, reg: Register) -> u8 {
        // assume upper address byte is 0
        let address = match reg {
            Register::X => {
                self.memory[self.program_counter] + self.idx_register_x
            },
            Register::Y => {
                self.memory[self.program_counter] + self.idx_register_y
            }
        };
        let tmp = self.memory[address as u16];
        self.program_counter += 1;
        tmp
    }

    fn get_zero_page_indirect(&mut self, reg: Register) -> u8 {
        let address = match reg {
            Register::X => {
                let indirect_address = self.memory[self.program_counter] + self.idx_register_x;
                u16::from_le_bytes([self.memory[indirect_address as u16], self.memory[indirect_address as u16 + 1]])
            },
            Register::Y => {
                let indirect_address = self.memory[self.program_counter];
                u16::from_le_bytes([self.memory[indirect_address as u16], self.memory[indirect_address as u16 + 1]]) + self.idx_register_y as u16
            }
        };
        let tmp = self.memory[address as u16];
        self.program_counter += 1;
        tmp
    }

    fn get_absolute(&mut self) -> u16 {
        let low = self.memory[self.program_counter] as u16;       // Fetch low byte
        let high = self.memory[self.program_counter + 1] as u16;  // Fetch high byte
        self.program_counter += 2;

        (high << 8) | low  // Combine into 16-bit address (little-endian)
    }

    fn get_indexed_absolute(&mut self, reg: Register) -> u16 {
        match reg {
            Register::X => self.get_absolute() + self.idx_register_x as u16,
            Register::Y => self.get_absolute() + self.idx_register_y as u16,
        }
    }
    fn get_absolute_indirect(&mut self) -> u16 {
        let pointer = self.get_absolute();  // Fetch pointer address
        let low = self.memory[pointer] as u16;

        // Handle 6502 page boundary bug
        let high = if pointer & 0xFF == 0xFF {
            self.memory[pointer & 0xFF00] as u16  // Wrap around to same page
        } else {
            self.memory[pointer + 1] as u16
        };

        (high << 8) | low  // Combine into final 16-bit address
    }



    pub fn or_immediate(&mut self) {
        self.do_or(self.get_immediate());
    }

    pub fn or_absolute(&mut self) {
        self.do_or(self.get_absolute());
    }

    #[inline]
    pub fn do_or(&mut self, data: u8) {
        self.accumulator |= data;
        *self.processor_status &= ((self.accumulator == 0) as u8 & *ProcessorStatusFlag::Zero) | (self.accumulator & *ProcessorStatusFlag::Negative);
    }

    pub fn noop(&mut self) {}
}


