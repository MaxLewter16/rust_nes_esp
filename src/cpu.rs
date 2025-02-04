use std::{io::Write, ops::{Deref, DerefMut, Index, IndexMut}};
use bitflags::bitflags;

use crate::opmap::OP_MAP;

// Primary Registers?
const STACK_RESET: u8 = 0xff;
const STACK: u16 = 0x0100;

// Memory Map constants
// constants specify the start of named section
pub const RAM: u16 = 0;
pub const MMIO: u16 = 0x2000;
pub const EXPANSION_ROM: u16 = 0x4020;
pub const SRAM: u16 = 0x6000;
pub const PROGRAM_ROM: u16 = 0x8000;

struct Program {
    file: Box<[u8]>,
}

impl Index<u16> for Program {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        &self.file[(address - PROGRAM_ROM) as usize]
    }
}

impl IndexMut<u16> for Program {
    fn index_mut(&mut self, address: u16) -> &mut Self::Output {
        &mut self.file[(address - 0x8000) as usize]
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ProcessorStatusFlags: u8 {
        const CARRY     = 1 << 0;
        const ZERO      = 1 << 1;
        const INTERRUPT = 1 << 2;  // IRQ disabled
        const DECIMAL   = 1 << 3;  // Not used on NES
        const BREAK     = 1 << 4;
        const UNUSED    = 1 << 5;  // Always set on NES
        const OVERFLOW  = 1 << 6;
        const NEGATIVE  = 1 << 7;
    }
}


pub struct CPU {
    pub memory: Memory,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub accumulator: u8,
    pub idx_register_x: u8,
    pub idx_register_y: u8,
    pub processor_status: ProcessorStatusFlags,
}

struct Memory {
    program: Program,
    ram: [u8; (MMIO - RAM) as usize],
}

impl Index<u16> for Memory {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        if address < MMIO { &self.ram[address as usize] }
        else if address < EXPANSION_ROM {self.mmio(address)}
        else if address < SRAM {
            //Expansion ROM
            &0u8
        }
        else if address < PROGRAM_ROM {
            //SRAM
            &0u8
        }
        else {
            &self.program[address]
        }
    //      Possible refactor, memory needs to be mirrored and this handles out of bounds address
    //     match address {
    //         0x0000..=0x1FFF => &self.ram[(address % 0x0800) as usize], // Mirror every 2 KB
    //         0x2000..=0x3FFF => self.mmio(0x2000 + (address % 8)), // Mirrors every 8 bytes
    //         0x6000..=0x7FFF => &0u8,  // SRAM (not yet implemented)
    //         0x8000..=0xFFFF => &self.program[address],
    //         _ => panic!("Invalid memory access: {:#06X}", address),
    // }
    }
}

impl Memory {
    fn write(&mut self, address: u16, data: u8) {
        if address < MMIO { self.ram[address as usize] = data }
        else if address < EXPANSION_ROM {self.mmio_write(address, data)}
        else if address < SRAM {
            //Expansion ROM
        }
        else if address < PROGRAM_ROM {
            //SRAM
        }
        else {
            self.program[address] = data
        }
    }

    fn from_program(mut program: Vec<u8>) -> Self {
        program.resize(0x10000 - PROGRAM_ROM as usize, 0);
        Memory { program: Program{file: program.into_boxed_slice()}, ram: [0u8; (MMIO - RAM) as usize]}
    }

    fn from_file(path: String) -> Self {
        unimplemented!()
    }

    fn mmio(&self, address: u16) -> &u8 {
        //TODO
        // MMIO_MAP[address]();
        unimplemented!()
    }

    fn mmio_write(&self, address: u16, data: u8) {
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
            program_counter: PROGRAM_ROM,
            stack_pointer: STACK_RESET,
            accumulator: 0,
            idx_register_x: 0,
            idx_register_y: 0,
            processor_status: ProcessorStatusFlags::from_bits_truncate(0b000000),
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

    //execute 'steps' instructions if steps is Some, otherwise run until program terminates
    pub fn execute(&mut self, steps: Option<usize>) {
        if let Some(steps) = steps {
            for _ in 0..steps {self.advance();}
        }
        else { loop {self.advance();} }
    }

    fn advance(&mut self) {
        let i = OP_MAP[self.memory[self.program_counter] as usize];
        self.program_counter += 1;
        i(self);
    }

    fn get_immediate(&mut self) -> u16 {
        let pc = self.program_counter;
        self.program_counter += 1;
        pc
    }

    fn get_zero_page(&mut self) -> u16 {
        let pc = self.program_counter;
        self.program_counter += 1;
        // assume upper address byte is 0
        self.memory[pc] as u16
    }

    fn get_zero_page_xy(&mut self, reg: Register) -> u16 {
        let pc = self.program_counter;
        // assume upper address byte is 0
        self.program_counter += 1;
        match reg {
            Register::X => {
                (self.memory[pc] + self.idx_register_x) as u16
            },
            Register::Y => {
                (self.memory[pc] + self.idx_register_y) as u16
            }
        }
    }

    fn get_zero_page_xy_indirect(&mut self, reg: Register) -> u16 {
        let pc = self.program_counter;
        self.program_counter += 1;
        match reg {
            Register::X => {
                let indirect_address = self.memory[pc] + self.idx_register_x;
                u16::from_le_bytes([self.memory[indirect_address as u16], self.memory[indirect_address as u16 + 1]])
            },
            Register::Y => {
                let indirect_address = self.memory[pc];
                u16::from_le_bytes([self.memory[indirect_address as u16], self.memory[indirect_address as u16 + 1]]) + self.idx_register_y as u16
            }
        }
    }

    /// Fetches an absolute address but does NOT return the value.
    fn get_absolute(&mut self) -> u16 {
        let low = self.memory[self.program_counter];
        let high = self.memory[self.program_counter + 1];

        self.program_counter += 2;

        u16::from_le_bytes([low, high])
    }

    /// Fetches an indexed absolute address (Absolute,X or Absolute,Y)
    fn get_absolute_xy(&mut self, reg: Register) -> u16 {
        let base_addr = self.get_absolute();
        match reg {
            Register::X => base_addr.wrapping_add(self.idx_register_x as u16),
            Register::Y => base_addr.wrapping_add(self.idx_register_y as u16),
        }
    }

    /// Fetches an absolute indirect address value(used for JMP (indirect)).
    fn get_absolute_indirect(&mut self) -> u16 {
        let addr_ptr = self.get_absolute();
        let low = self.memory[addr_ptr];
        let high = self.memory[addr_ptr.wrapping_add(1)];

        u16::from_le_bytes([low, high])
    }

    /*
        store instruction
     */

    pub fn store_a_absolute(&mut self) {
        let address = self.get_absolute();
        self.store(address, self.accumulator);
    }

    pub fn store_a_absolute_x(&mut self) {
        let address = self.get_absolute_xy(Register::X);
        self.store(address, self.accumulator);
    }

    pub fn store_a_absolute_y(&mut self) {
        let address = self.get_absolute_xy(Register::Y);
        self.store(address, self.accumulator);
    }

    pub fn store_a_zero_page(&mut self) {
        let address = self.get_zero_page();
        self.store(address, self.accumulator);
    }

    pub fn store_a_zero_page_x(&mut self) {
        let address = self.get_zero_page_xy(Register::X);
        self.store(address, self.accumulator);
    }

    pub fn store_a_zero_page_x_indirect(&mut self) {
        let address = self.get_zero_page_xy_indirect(Register::X);
        self.store(address, self.accumulator);
    }

    pub fn store_a_zero_page_y_indirect(&mut self) {
        let address = self.get_zero_page_xy_indirect(Register::Y);
        self.store(address, self.accumulator);
    }

    pub fn store_x_absolute(&mut self) {
        let address = self.get_absolute();
        self.store(address, self.idx_register_x);
    }

    pub fn store_x_zero_page(&mut self) {
        let address = self.get_zero_page();
        self.store(address, self.idx_register_x);
    }

    pub fn store_x_zero_page_y(&mut self) {
        let address = self.get_zero_page_xy(Register::Y);
        self.store(address, self.idx_register_x);
    }

    pub fn store_y_absolute(&mut self) {
        let address = self.get_absolute();
        self.store(address, self.idx_register_y);
    }

    pub fn store_y_zero_page(&mut self) {
        let address = self.get_zero_page();
        self.store(address, self.idx_register_y);
    }

    pub fn store_y_zero_page_x(&mut self) {
        let address = self.get_zero_page_xy(Register::X);
        self.store(address, self.idx_register_y);
    }

    #[inline]
    pub fn store(&mut self, address: u16, data: u8) {
        self.memory.write(address, data);
    }

    /*
        or instruction
     */

    pub fn or_immediate(&mut self) {
        let address = self.get_immediate();
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_absolute(&mut self) {
        let address = self.get_absolute();
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_absolute_x(&mut self) {
        let address = self.get_absolute_xy(Register::X);
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_absolute_y(&mut self) {
        let address = self.get_absolute_xy(Register::Y);
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_zero_page(&mut self) {
        let address = self.get_zero_page();
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_zero_page_x(&mut self) {
        let address = self.get_zero_page_xy(Register::X);
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_zero_page_x_indirect(&mut self) {
        let address = self.get_zero_page_xy_indirect(Register::X);
        let data = self.memory[address];
        self.or(data);
    }

    pub fn or_zero_page_y_indirect(&mut self) {
        let address = self.get_zero_page_xy_indirect(Register::Y);
        let data = self.memory[address];
        self.or(data);
    }

    #[inline]
    pub fn or(&mut self, data: u8) {
        self.accumulator |= data;
        //clear relevant flags
        self.processor_status &= !(ProcessorStatusFlags::ZERO | ProcessorStatusFlags::NEGATIVE);
        //set flags
        self.processor_status |= (if self.accumulator == 0 {ProcessorStatusFlags::ZERO} else {ProcessorStatusFlags::empty()}) | (ProcessorStatusFlags::from_bits_truncate(self.accumulator & ProcessorStatusFlags::NEGATIVE.bits()));
    }

    /*
        and instruction
    */

    #[inline]
    pub fn and(&mut self, data: u8) {
        self.accumulator &= data;
        //clear relevant flags
        self.processor_status &= !(ProcessorStatusFlags::ZERO | ProcessorStatusFlags::NEGATIVE);
        //set flags
        self.processor_status |= (if self.accumulator == 0 {ProcessorStatusFlags::ZERO} else {ProcessorStatusFlags::empty()}) | (ProcessorStatusFlags::from_bits_truncate(self.accumulator & ProcessorStatusFlags::NEGATIVE.bits()));
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
        assert_eq!(cpu.processor_status, ProcessorStatusFlags::NEGATIVE);
    }

    #[test]
    fn test_simple_store_ram() {
        let mut instr = vec![0x09, 0xaa];
        for i in 0..1<<7 {
            instr.push(0x8d);
            let a = (i * (MMIO / (1<<7)) as u16).to_le_bytes();
            instr.push(a[0]);
            instr.push(a[1]);
        }
        let len = instr.len();
        let mut cpu = CPU::with_program(instr);
        for _ in 0..len {
            cpu.advance();
        }
        for i in 0..1<<7 {
            let a: u16 = i * (MMIO / (1<<7));
            assert_eq!(cpu.memory[a], 0xaa);
        }
    }

    #[test]
    fn test_addressing() {
        //! this test depends on 'or', 'store', and 'transfer' instructions
        //test absolute
        let mut cpu = CPU::with_program(vec![0x09, 0xaa, 0x8d, 0xff, 0x10]);
        cpu.execute(Some(2));
        assert_eq!(cpu.memory[0x10ff], 0xaa);

        //test zero page
        let mut cpu = CPU::with_program(vec![0x09, 0xaa, 0x85, 0xff]);
        cpu.execute(Some(2));
        assert_eq!(cpu.memory[0x00ff], 0xaa);

        //TODO need 'transfer' instructions to get nontrivial values in X/Y
    }
}
