use std::{ops::{Deref, DerefMut, Index}};

use crate::opmap::OP_MAP;

// Primary Registers?
const STACK_RESET: u8 = 0xff;
const STACK: u16 = 0x0100;

struct Program {
    file: Box<[u8]>,
}

impl Index<u16> for Program {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        &self.file[(address - 0x8000) as usize]
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
    pub fn set(&mut self, flag: ProcessorStatusFlag) {
        self.flags |= flag as u8;
    }
    pub fn clear(&mut self, flag: ProcessorStatusFlag) {
        self.flags &= !(flag as u8);
    }
}

impl Default for ProcessorStatus {
    fn default() -> Self {
        Self { flags: 0 }
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
    ram: [u8; 0x2000],
}

impl Index<u16> for Memory {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        if address < 0x2000 { &self.ram[address as usize] }
        else if address < 0x4020 {self.mmio(address)}
        else if address < 0x6000 {
            //Expansion ROM
            &0u8
        }
        else if address < 0x8000 {
            //SRAM
            &0u8
        }
        else {
            &self.program[address]
        }
    }
}

impl Memory {
    fn from_program(mut program: Vec<u8>) -> Self {
        program.resize(0x10000-0x8000, 0);
        Memory { program: Program{file: program.into_boxed_slice()}, ram: [0u8; 0x2000]}
    }

    fn from_file(path: String) -> Self {
        unimplemented!()
    }

    fn mmio(&self, address: u16) -> &u8 {
        //TODO
        // MMIO_MAP[address]();
        unimplemented!()
    }
}

enum Register {
    X,
    Y
}

impl CPU {
    pub fn with_program(program: Vec<u8>) -> Self {
        CPU {
            memory: Memory::from_program(program),
            program_counter: 0x8000,
            stack_pointer: 0,
            accumulator: 0,
            idx_register_x: 0,
            idx_register_y: 0,
            processor_status: ProcessorStatus::default(),
        }
    }

    // pub fn new() -> Self {
    //     CPU {
    //         memory: Memory::from_file(""),
    //         program_counter: 0,
    //         idx_register_x: 0,
    //         idx_register_y: 0,
    //         processor_status: ProcessorStatus::new(),
    //         stack_pointer: STACK_RESET,
    //         accumulator: 0
    //     }
    // }

    fn advance(&mut self) {
        let i = OP_MAP[self.memory[self.program_counter] as usize];
        self.program_counter += 1;
        i(self);
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

    fn get_zero_page_xy_indirect(&mut self, reg: Register) -> u8 {
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
    /// Fetches an absolute address but does NOT return the value.
    fn get_absolute_address(&mut self) -> u16 {
        let low = self.memory[self.program_counter];
        let high = self.memory[self.program_counter + 1];

        self.program_counter += 2;

        u16::from_le_bytes([low, high])
    }

    // Fetch the value at the absolute address
    fn get_absolute(&mut self) -> u8 {
        let address = self.get_absolute_address();
        self.memory[address]
    }

    /// Fetches an indexed absolute address (Absolute,X or Absolute,Y) and returns the value stored at that address.
    fn get_absolute_xy(&mut self, reg: Register) -> u8 {
        let base_addr = self.get_absolute_address();
        let indexed_addr = match reg {
            Register::X => base_addr.wrapping_add(self.idx_register_x as u16),
            Register::Y => base_addr.wrapping_add(self.idx_register_y as u16),
        };

        self.memory[indexed_addr]
    }

    /// Fetches an absolute indirect address value(used for JMP (indirect)).
    fn get_absolute_indirect(&mut self) -> u8 {
        let addr_ptr = self.get_absolute_address();
        let low = self.memory[addr_ptr];
        let high = self.memory[addr_ptr.wrapping_add(1)];

        let target_addr = u16::from_le_bytes([low, high]);
        self.memory[target_addr]
    }

    pub fn or_immediate(&mut self) {
        let data = self.get_immediate();
        self.do_or(data);
    }

    pub fn or_absolute(&mut self) {
        let data = self.get_absolute();
        self.do_or(data);
    }

    pub fn or_absolute_x(&mut self) {
        let data = self.get_absolute_xy(Register::X);
        self.do_or(data);
    }

    pub fn or_absolute_y(&mut self) {
        let data = self.get_absolute_xy(Register::Y);
        self.do_or(data);
    }

    pub fn or_zero_page(&mut self) {
        let data = self.get_zero_page();
        self.do_or(data);
    }

    pub fn or_zero_page_x(&mut self) {
        let data = self.get_zero_page_xy(Register::X);
        self.do_or(data);
    }

    pub fn or_zero_page_x_indirect(&mut self) {
        let data = self.get_zero_page_xy_indirect(Register::X);
        self.do_or(data);
    }

    pub fn or_zero_page_y_indirect(&mut self) {
        let data = self.get_zero_page_xy_indirect(Register::Y);
        self.do_or(data);
    }

    #[inline]
    pub fn do_or(&mut self, data: u8) {
        self.accumulator |= data;
        //clear relevant flags
        *self.processor_status &= !(*ProcessorStatusFlag::Zero | *ProcessorStatusFlag::Negative);
        //set flags
        *self.processor_status |= (if self.accumulator == 0 {*ProcessorStatusFlag::Zero} else {0}) | (self.accumulator & *ProcessorStatusFlag::Negative);
    }

    pub fn noop(&mut self) {}
}

mod tests {
    use super::*;

    #[test]
    fn test_simple_or() {
        // or 0xaa into Accumulator
        let mut cpu = CPU::with_program(vec![0x09, 0xaa]);
        cpu.advance();
        assert_eq!(cpu.accumulator, 0xaa);
        assert_eq!(cpu.program_counter, 0x8002);
        assert_eq!(*cpu.processor_status, *ProcessorStatusFlag::Negative);
    }
}

