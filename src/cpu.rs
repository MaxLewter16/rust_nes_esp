use std::{io::{self, Read, Write}, marker::PhantomPinned, ops::{Deref, DerefMut, Index, IndexMut}, ptr::NonNull, u16};
use std::result::Result;
use std::fmt;
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
pub const PROGRAM_ROM_SIZE: u16 = 16 * (1 << 10);
pub const PROGRAM_ROM_2: u16 = PROGRAM_ROM + PROGRAM_ROM_SIZE;

struct ROM {
    file: Box<[u8]>,
    start_address: u16,
}

impl Index<u16> for ROM {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        &self.file[(address - self.start_address) as usize]
    }
}

impl IndexMut<u16> for ROM {
    fn index_mut(&mut self, address: u16) -> &mut Self::Output {
        &mut self.file[(address - self.start_address) as usize]
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ProcessorStatusFlags: u8 {
        const CARRY     = 1 << 0;
        const ZERO      = 1 << 1;
        const INTERRUPT = 1 << 2;
        const DECIMAL   = 1 << 3;  // Not used on NES
        const BREAK     = 1 << 4;
        const UNUSED    = 1 << 5;  // Always set on NES
        const OVERFLOW  = 1 << 6;
        const NEGATIVE  = 1 << 7;
    }
}

impl fmt::Display for ProcessorStatusFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "N:{} V:{} -:{} B:{} D:{} I:{} Z:{} C:{}",
            self.contains(ProcessorStatusFlags::NEGATIVE) as u8,
            self.contains(ProcessorStatusFlags::OVERFLOW) as u8,
            self.contains(ProcessorStatusFlags::UNUSED) as u8,  // Unused bit
            self.contains(ProcessorStatusFlags::BREAK) as u8,
            self.contains(ProcessorStatusFlags::DECIMAL) as u8,
            self.contains(ProcessorStatusFlags::INTERRUPT) as u8,
            self.contains(ProcessorStatusFlags::ZERO) as u8,
            self.contains(ProcessorStatusFlags::CARRY) as u8
        )
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
    program_rom: Vec<ROM>,
    vrom: Vec<ROM>,
    /* Memory must uphold the following:
        - active_program_1/2 must be non-null
        - active_program_1/2 should not be used to modify program memory
       Because reading program rom occurs every emulated cycle it should have
       minimal overhead, which is achieved with a pointer to the active memory.
    */
    active_program_1: NonNull<ROM>,
    active_program_2: NonNull<ROM>,
    // because Memory is contains pointers to itself it can't be moved
    _phantom_pin: PhantomPinned,
    ram: [u8; (MMIO - RAM) as usize],
    mapper: u8, //TODO should be enum probably
}

impl Index<u16> for Memory {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        match address {
            RAM..MMIO => &self.ram[(address % 0x0800) as usize], // Mirror every 2 KB
            MMIO..EXPANSION_ROM => self.mmio(address % 8), // Mirrors every 8 bytes
            EXPANSION_ROM..SRAM => &0u8, //EXPANSION_ROM
            SRAM..PROGRAM_ROM => &0u8, // SRAM (not yet implemented)
            // this is safe because active program roms are always selected
            PROGRAM_ROM..PROGRAM_ROM_2 => unsafe{&self.active_program_1.as_ref()[address]},
            PROGRAM_ROM_2..=u16::MAX => unsafe{&self.active_program_2.as_ref()[address]},
        }
    }
}

pub enum NesError {
    IO(io::Error),
    FileFormat(&'static str)
}

impl From<io::Error> for NesError {
    fn from(value: io::Error) -> Self {
        NesError::IO(value)
    }
}

impl Memory {
    fn write(&mut self, address: u16, data: u8) {
        match address {
            RAM..MMIO => self.ram[(address % 0x0800) as usize] = data, // Mirror every 2 KB
            MMIO..EXPANSION_ROM => self.mmio_write(address % 8, data), // Mirrors every 8 bytes
            EXPANSION_ROM..SRAM => (), //EXPANSION_ROM
            SRAM..PROGRAM_ROM => (), // SRAM (not yet implemented)
            // TODO: writes to program rom are used to control memory mappers
            PROGRAM_ROM..PROGRAM_ROM_2 => (),
            PROGRAM_ROM_2..=u16::MAX => (),
        }
    }

    fn from_program(mut program: Vec<u8>) -> Self {
        program.resize(0x10000 - PROGRAM_ROM as usize, 0);
        let mut program = ROM{file: program.into_boxed_slice(),start_address: PROGRAM_ROM};
        let ap1 = NonNull::new(&mut program).unwrap();
        let ap2 = NonNull::new(&mut program).unwrap();
        Memory {
            program_rom: vec![program],
            vrom: vec![],
            active_program_1: ap1,
            active_program_2: ap2,
            ram: [0u8; (MMIO - RAM) as usize],
            mapper: 0,
            _phantom_pin: PhantomPinned
        }
    }

    fn from_file(path: String) -> Result<Self, NesError> {
        let mut file = std::fs::File::open(path)?;
        let mut header = [0u8; 16];
        if file.read(&mut header)? < 16 {return Err(NesError::FileFormat("file too short"))};
        if header[0..4] != ['N' as u8, 'E' as u8, 'S' as u8, 0x1a] {
            return Err(NesError::FileFormat("incorrect identifying bytes, not a .nes file?"))
        };

        let prg_rom_count = header[4];
        let vrom_count = header[5];
        let rom_control = &header[6..7];
        let ram_bank_count = header[8];

        let mapper_number = (rom_control[1] & 0xf0) | (rom_control[0] >> 4);
        let mirroring_type = (header[0] & 1) != 0;
        let battery_ram = (header[0] & 2) != 0;
        let trainer = (header[0] & 4) != 0;

        let mut program = Vec::new();
        let mut vrom = Vec::new();

        for _ in 0..prg_rom_count {
            let mut prg_rom_buf = Box::new([0u8; 16 * (1 << 10)]);
            file.read_exact(prg_rom_buf.as_mut_slice())?;
            program.push(ROM{file: prg_rom_buf, start_address: PROGRAM_ROM})
        }

        for _ in 0..vrom_count {
            let mut vrom_buf = Box::new([0u8; 8 * (1 << 10)]);
            file.read_exact(vrom_buf.as_mut_slice())?;
            // ! VROM goes in PPU which is unimplemented
            vrom.push(ROM{file: vrom_buf, start_address: EXPANSION_ROM})
        }

        let active_program_1 = NonNull::new(&mut program[0]).unwrap();
        let active_program_2 = NonNull::new(&mut program[0]).unwrap();

        Ok(Memory{
            program_rom: program,
            active_program_1,
            active_program_2,
            vrom,
            ram: [0u8; (MMIO - RAM) as usize],
            mapper: mapper_number,
            _phantom_pin: PhantomPinned
        })
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

    fn get_zero_page_x(&mut self) ->u16{
        let pc = self.program_counter;
        // assume upper address byte is 0
        self.program_counter += 1;
        self.memory[pc].wrapping_add(self.idx_register_x) as u16
    }

    fn get_zero_page_y(&mut self) ->u16{
        let pc = self.program_counter;
        // assume upper address byte is 0
        self.program_counter += 1;
        self.memory[pc].wrapping_add(self.idx_register_y) as u16
    }

    fn get_zero_page_x_indirect(&mut self) -> u16 {
        let pc = self.program_counter;
        self.program_counter += 1;
        let indirect_address = self.memory[pc].wrapping_add(self.idx_register_x);
        u16::from_le_bytes([self.memory[indirect_address as u16], self.memory[indirect_address as u16 + 1]])
    }

    fn get_zero_page_y_indirect(&mut self) -> u16 {
        let pc = self.program_counter;
        self.program_counter += 1;
        let indirect_address = self.memory[pc];
        u16::from_le_bytes([self.memory[indirect_address as u16], self.memory[indirect_address as u16 + 1]]).wrapping_add(self.idx_register_y as u16)
    }

    /// Fetches an absolute address but does NOT return the value.
    fn get_absolute(&mut self) -> u16 {
        let low = self.memory[self.program_counter];
        let high = self.memory[self.program_counter + 1];

        self.program_counter += 2;

        u16::from_le_bytes([low, high])
    }

    fn get_absolute_x(&mut self) -> u16 {
        let base_addr = self.get_absolute();
        base_addr.wrapping_add(self.idx_register_x as u16)
    }

    fn get_absolute_y(&mut self) -> u16 {
        let base_addr = self.get_absolute();
        base_addr.wrapping_add(self.idx_register_y as u16)
    }

    /// Fetches an absolute indirect address value(used for JMP (indirect)).
    fn get_absolute_indirect(&mut self) -> u16 {
        let addr_ptr = self.get_absolute();
        let low = self.memory[addr_ptr];
        let high = self.memory[addr_ptr.wrapping_add(1)];

        u16::from_le_bytes([low, high])
    }

    fn get_relative(&mut self) -> u16 {
        let offset = (self.memory[self.program_counter] as i8) as i16;
        self.program_counter += 1;
        //? should it be allowed to branch outside of program memory
        self.program_counter.wrapping_add(offset as u16)
    }

    pub fn noop(&mut self) {}

    pub fn transfer_x_sp(&mut self) {
        self.stack_pointer = self.idx_register_x;
    }

    pub fn load_m_a_immediate(&mut self) {
        let address = self.get_immediate();
        self.accumulator = self.memory[address];
        self.update_negative_zero_flags(self.accumulator);
    }

    #[inline]
    // set NEGATIVE flag if 'test' is negative, reset otherwise
    // set ZERO flag if 'test' is zero, reset otherwise
    pub fn update_negative_zero_flags(&mut self, test: u8) {
         //clear relevant flags
         self.processor_status &= !(ProcessorStatusFlags::ZERO | ProcessorStatusFlags::NEGATIVE);
         //set flags
         self.processor_status |=
             (if self.accumulator == 0 {ProcessorStatusFlags::ZERO} else {ProcessorStatusFlags::empty()}) |
             (ProcessorStatusFlags::from_bits_truncate(self.accumulator & ProcessorStatusFlags::NEGATIVE.bits()));
    }

}

/*
    transfer instructions
*/
// Does not work for 'transfer X to SP' instruction
macro_rules! transfer_gen {
    ($name: ident, $source: ident, $target: ident) => {
        impl CPU {
            pub fn $name(&mut self) {
                self.$target = self.$source;
                self.update_negative_zero_flags(self.$target);
            }
        }
    };
}
transfer_gen!(transfer_a_x, accumulator, idx_register_x);
transfer_gen!(transfer_x_a, idx_register_x, accumulator);
transfer_gen!(transfer_a_y, accumulator, idx_register_y);
transfer_gen!(transfer_y_a, idx_register_y, accumulator);
transfer_gen!(transfer_sp_x, stack_pointer, idx_register_x);

/*
    load instructions
*/
macro_rules! load_gen {
    ($name: ident, $addressing_mode: ident, $target: ident) => {
        impl CPU {
            pub fn $name(&mut self) {
                let address = self.$addressing_mode();
                self.$target = self.memory[address];
                self.update_negative_zero_flags(self.$target);
            }
        }
    };
}
load_gen!(load_a_immediate, get_immediate, accumulator);
load_gen!(load_a_absolute, get_absolute, accumulator);
load_gen!(load_a_absolute_x, get_absolute_x, accumulator);
load_gen!(load_a_absolute_y, get_absolute_y, accumulator);
load_gen!(load_a_zero_page, get_zero_page, accumulator);
load_gen!(load_a_zero_page_x, get_zero_page_x, accumulator);
load_gen!(load_a_zero_page_x_indirect, get_zero_page_x_indirect, accumulator);
load_gen!(load_a_zero_page_y_indirect, get_zero_page_y_indirect, accumulator);

load_gen!(load_x_immediate, get_immediate, idx_register_x);
load_gen!(load_x_absolute, get_absolute, idx_register_x);
load_gen!(load_x_absolute_y, get_absolute_y, idx_register_x);
load_gen!(load_x_zero_page, get_zero_page, idx_register_x);
load_gen!(load_x_zero_page_y, get_zero_page_y, idx_register_x);

load_gen!(load_y_immediate, get_immediate, idx_register_y);
load_gen!(load_y_absolute, get_absolute, idx_register_y);
load_gen!(load_y_absolute_x, get_absolute_x, idx_register_y);
load_gen!(load_y_zero_page, get_zero_page, idx_register_y);
load_gen!(load_y_zero_page_x, get_zero_page_x, idx_register_y);

/*
    branch instructions
*/
macro_rules! branch_gen {
    ($name: ident, $inverse_name: ident, $flag: expr) => {
        impl CPU {
            pub fn $name(&mut self) {
                if self.processor_status.contains($flag) {
                    self.program_counter = self.get_relative();
                }
            }

            pub fn $inverse_name(&mut self) {
                if !self.processor_status.contains($flag) {
                    self.program_counter = self.get_relative();
                }
            }
        }
    };
}
branch_gen!(branch_on_zero_set, branch_on_zero_reset, ProcessorStatusFlags::ZERO);
branch_gen!(branch_on_carry_set, branch_on_carry_reset, ProcessorStatusFlags::CARRY);
branch_gen!(branch_on_negative_set, branch_on_negative_reset, ProcessorStatusFlags::NEGATIVE);
branch_gen!(branch_on_overflow_set, branch_on_overflow_reset, ProcessorStatusFlags::OVERFLOW);

/*
    store instructions
*/
macro_rules! store_gen {
    ($name: ident, $p: path, $register:ident) => {
        impl CPU {
            pub fn $name(&mut self) {
                let address = $p(self);
                self.memory.write(address, self.$register)
            }
        }
    };
}
// store for accumulator
store_gen!(store_a_absolute, CPU::get_absolute, accumulator);
store_gen!(store_a_absolute_x, CPU::get_absolute_x, accumulator);
store_gen!(store_a_absolute_y, CPU::get_absolute_y, accumulator);
store_gen!(store_a_zero_page, CPU::get_zero_page, accumulator);
store_gen!(store_a_zero_page_x, CPU::get_zero_page_x, accumulator);
store_gen!(store_a_zero_page_y, CPU::get_zero_page_y, accumulator);
store_gen!(store_a_zero_page_x_indirect, CPU::get_zero_page_x_indirect, accumulator);
store_gen!(store_a_zero_page_y_indirect, CPU::get_zero_page_y_indirect, accumulator);

// store for reg x
store_gen!(store_x_absolute, CPU::get_absolute, idx_register_x);
store_gen!(store_x_zero_page, CPU::get_zero_page, idx_register_x);
store_gen!(store_x_zero_page_y, CPU::get_zero_page_y, idx_register_x);

// store for reg y
store_gen!(store_y_absolute, CPU::get_absolute, idx_register_y);
store_gen!(store_y_zero_page, CPU::get_zero_page, idx_register_y);
store_gen!(store_y_zero_page_x, CPU::get_zero_page_x, idx_register_y);

/*
    or instructions
*/
macro_rules! or_gen {
    ($name: ident, $p: path) => {
        impl CPU {
            pub fn $name(&mut self) {
                let address = $p(self);
                let data = self.memory[address];
                self.accumulator |= data;
                self.update_negative_zero_flags(self.accumulator);
            }
        }
    };
}
or_gen!(or_immediate, CPU::get_immediate);
or_gen!(or_absolute, CPU::get_absolute);
or_gen!(or_absolute_x, CPU::get_absolute_x);
or_gen!(or_absolute_y, CPU::get_absolute_y);
or_gen!(or_zero_page, CPU::get_zero_page);
or_gen!(or_zero_page_x, CPU::get_zero_page_x);
or_gen!(or_zero_page_x_indirect, CPU::get_zero_page_x_indirect);
or_gen!(or_zero_page_y_indirect, CPU::get_zero_page_y_indirect);

/*
    and instructions
*/
macro_rules! and_gen {
    ($name: ident, $p: path) => {
        impl CPU {
            pub fn $name(&mut self) {
                let address = $p(self);
                let data = self.memory[address];
                self.accumulator &= data;
                self.update_negative_zero_flags(self.accumulator);
            }
        }
    };
}
and_gen!(and_immediate, CPU::get_immediate);
and_gen!(and_absolute, CPU::get_absolute);
and_gen!(and_absolute_x, CPU::get_absolute_x);
and_gen!(and_absolute_y, CPU::get_absolute_y);
and_gen!(and_zero_page, CPU::get_zero_page);
and_gen!(and_zero_page_x, CPU::get_zero_page_x);
and_gen!(and_zero_page_x_indirect, CPU::get_zero_page_x_indirect);
and_gen!(and_zero_page_y_indirect, CPU::get_zero_page_y_indirect);

macro_rules! clear_flag_gen {
    ($name:ident, $flag:expr) => {
        impl CPU {
            pub fn $name(&mut self) {
                self.processor_status &= !$flag;
            }
        }
    };
}
clear_flag_gen!(clear_carry, ProcessorStatusFlags::CARRY);
clear_flag_gen!(clear_decimal, ProcessorStatusFlags::DECIMAL);
clear_flag_gen!(clear_interrupt, ProcessorStatusFlags::INTERRUPT);
clear_flag_gen!(clear_overflow, ProcessorStatusFlags::OVERFLOW);

macro_rules! set_flag_gen {
    ($name:ident, $flag:expr) => {
        impl CPU {
            pub fn $name(&mut self) {
                self.processor_status |= $flag;
            }
        }
    };
}
set_flag_gen!(set_carry, ProcessorStatusFlags::CARRY);
set_flag_gen!(set_decimal, ProcessorStatusFlags::DECIMAL);
set_flag_gen!(set_interrupt, ProcessorStatusFlags::INTERRUPT);

mod tests {
    use super::*;

    #[test]
    // tests or, lda, ldx, ldy
    fn test_baseline() {
        // or 0xaa into Accumulator
        let mut cpu = CPU::with_program(vec![0x09, 0xaa]);
        cpu.advance();
        assert_eq!(cpu.accumulator, 0xaa);
        assert_eq!(cpu.program_counter, 0x8002);
        assert_eq!(cpu.processor_status, ProcessorStatusFlags::NEGATIVE);

        let mut cpu = CPU::with_program(vec![0xa9, 0xbb, 0xa2, 0xbb, 0xa0, 0xbb]);
        cpu.execute(Some(3));
        assert_eq!(cpu.accumulator, 0xbb);
        assert_eq!(cpu.idx_register_x, 0xbb);
        assert_eq!(cpu.idx_register_y, 0xbb);
    }

    #[test]
    fn test_simple_and() {
        let mut cpu = CPU::with_program(vec![0x29, 0xaa]);
        cpu.advance();
        assert_eq!(cpu.accumulator, 0x00); // Fix: AND results in 0x00
        assert_eq!(cpu.program_counter, 0x8002);
        assert_eq!(cpu.processor_status, ProcessorStatusFlags::ZERO); // Fix: Expect ZERO, not NEGATIVE
    }

    #[test]
    fn test_simple_and_neg() {
        let mut cpu = CPU::with_program(vec![0xA9, 0xFF, 0x29, 0xAA]); // LDA #0xFF, AND #0xAA
        cpu.execute(Some(2));
        assert_eq!(cpu.accumulator, 0xAA);
        assert_eq!(cpu.program_counter, 0x8004);
        assert_eq!(cpu.processor_status, ProcessorStatusFlags::NEGATIVE);
    }

    #[test]
    fn test_transfer() {
        // ora 0xaa
        // txa
        // txy
        // tsp
        // lda 0xbb
        // txa
        // lda 0xbb
        // tya
        let mut cpu = CPU::with_program(vec![0x09, 0xaa, 0xaa, 0xa8, 0x9a, 0xa9, 0xbb, 0x8a, 0xa9, 0xbb, 0x98, 0xa2, 0xbb, 0xba]);
        cpu.execute(Some(4));
        assert!(cpu.idx_register_x == 0xaa && cpu.idx_register_y == 0xaa && cpu.stack_pointer == 0xaa);
        cpu.execute(Some(2));
        assert!(cpu.accumulator == 0xaa);
        cpu.execute(Some(2));
        assert!(cpu.accumulator == 0xaa);
        cpu.execute(Some(2));
        assert!(cpu.stack_pointer == 0xaa);
        assert!(cpu.idx_register_x == 0xaa);
    }

    #[test]
    fn test_loads() {

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

        //test zero page x
        let mut cpu = CPU::with_program(vec![0xa9, 0xaa, 0xa2, 0xf0, 0x95, 0x0f, 0xa9, 0x00, 0xb5, 0x0f]);
        cpu.execute(Some(5));
        assert!(cpu.memory[0xff] == 0xaa);
        assert_eq!(cpu.accumulator, 0xaa);

        //test zero page y
        //lda 0xaa
        //ldx 0xf0
        //str 0xf(x)
        //ldy 0xf0
        //ld  0xf(y)
        let mut cpu = CPU::with_program(vec![0xa9, 0xaa, 0xa2, 0xf0, 0x95, 0x0f, 0xa0, 0xf0, 0xb6, 0x0f]);
        cpu.execute(Some(5));
        assert!(cpu.memory[0xff] == 0xaa);
        assert_eq!(cpu.idx_register_x, 0xaa);

        //test absolute y
        /*
        lda #$aa
        ldy #$ff
        sta $1001, y
         */
        let mut cpu = CPU::with_program(vec![0xa9, 0xaa, 0xa0, 0xff, 0x99, 0x01, 0x10]);
        cpu.execute(Some(3));
        assert!(cpu.memory[0x1100] == 0xaa);

        //test absolute x
        /*
        lda #$aa
        ldx #$ff
        sta $1001, x
        */
        let mut cpu = CPU::with_program(vec![0xa9,0xaa,0xa2,0xff,0x9d,0x01,0x10]);
        cpu.execute(Some(3));
        assert!(cpu.memory[0x1100] == 0xaa);

        //test absolute indirect


        //test zero-page x indirect
        /*
        lda #$aa
        sta $cc
        ldx #$0c
        sta ($c0, x)
         */
        let mut cpu = CPU::with_program(vec![ 0xa9, 0xaa, 0x85, 0xcc, 0xa2, 0x0c, 0x81, 0xc0 ]);
        cpu.execute(Some(4));
        assert!(cpu.memory[0xaa] == 0xaa);

        //test zero-page y indirect
        /*
        lda #$aa
        sta $c0
        ldy #$0c
        sta ($c0), y
         */
        let mut cpu = CPU::with_program(vec![0xa9, 0xaa, 0x85, 0xc0, 0xa0, 0x0c, 0x91, 0xc0 ]);
        cpu.execute(Some(4));
        assert!(cpu.memory[0xb6] == 0xaa);

        //test relative, use branch on carry reset
        //branch forward by maximum offset 3 times, branch back by max offset 3 times
        let mut instr: Vec<u8> = Vec::new();
        let mut address = 0;
        instr.resize(0x200, 0);
        instr[0x0..0x2].copy_from_slice(&[0x90, 0x7f]);
        address += 0x7f + 0x2;
        instr[address..address + 0x4].copy_from_slice(&[0x09, 0x01, 0x90, 0x7f]);
        address += 0x7f + 0x4;
        instr[address..address + 0xd].copy_from_slice(&[0x09, 0x02, 0x90, 0x07, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x90, 0x80]);
        address = address + 0xd - 0x80;
        instr[address..address + 0x4].copy_from_slice(&[0x09, 0x04, 0x90, 0x80]);
        address = address + 0x4 - 0x80;
        instr[address..address + 0x4].copy_from_slice(&[0x09, 0x08, 0x90, 0x80]);
        let mut cpu = CPU::with_program(instr);
        cpu.execute(Some(9));
        assert_eq!(cpu.accumulator, 0x0f);



        //TODO absolute indirect (jmp instruction)
    }

    #[test]
    fn test_flag_set_reset() {
        let mut cpu = CPU::with_program(vec![0x38, 0xf8, 0x78, 0x18, 0xd8, 0x58]);
        cpu.execute(Some(3));
        assert!(cpu.processor_status.contains(ProcessorStatusFlags::CARRY | ProcessorStatusFlags::DECIMAL | ProcessorStatusFlags::INTERRUPT));
        cpu.execute(Some(3));
        assert!((!cpu.processor_status).contains(ProcessorStatusFlags::CARRY | ProcessorStatusFlags::DECIMAL | ProcessorStatusFlags::INTERRUPT));

        //TODO test clear overflow
    }
    macro_rules! test_and_or_instruction {
        ($name:ident, $num_programs:expr, $program:expr, $initial_a:expr, $expected_a:expr) => {
            #[test]
            fn $name() {
                let mut cpu = CPU::with_program($program.to_vec());

                cpu.accumulator = $initial_a;

                cpu.execute(Some($num_programs));

                // Verify accumulator result
                assert_eq!(cpu.accumulator, $expected_a, "Accumulator incorrect: expected {:08b}, got {:08b}", $expected_a, cpu.accumulator);
            }
        };
    }
    // AND instructions

    // and zero page (Opcode: 0x25)
    test_and_or_instruction!(test_and_zero_page, 3, [0x85, 0x50, 0xA9, 0b00001010, 0x25, 0x50], 0b10101010, 0b00001010); // Set accumulator to 0b10101010, STA: 0x50, LDA: 0b00001010, AND 0x50
    // and zero page x (Opcode: 0x35)
    test_and_or_instruction!(test_and_zero_page_x, 4, [0xa2, 0x50, 0x8d, 0x50, 0x00, 0xA9, 0b00001010, 0x35, 0x00], 0b10101010, 0b00001010); // Set accumulator to 0b10101010, LDX: 0x50, STA: 0x50, LDA: 0b00001010, AND 0x00 x
    // and abs (Opcode: 0x2D)
    test_and_or_instruction!(test_and_absolute, 3, [0xa2, 0x50, 0x00, 0xA9, 0b00001010, 0x2D, 0x50, 0x00], 0b10101010, 0b00001010); // Set accumulator to 0b10101010, STA: 0x50, LDA: 0b00001010, AND 0x0050
    // and abs X (Opcode: 0x3D)
    test_and_or_instruction!(test_and_absolute_x, 4, [0xa2, 0x50, 0x8d, 0x50, 0x00, 0xA9, 0b00001010, 0x3D, 0x00, 0x00], 0b10101010, 0b00001010); // Set accumulator to 0b10101010, LDX: 0x50, STA: 0x50, LDA: 0b00001010, AND 0x0000 x
    // and abs Y (Opcode: 0x39)
    test_and_or_instruction!(test_and_absolute_y, 4, [0xa0, 0x50, 0x8d, 0x50, 0x00, 0xA9, 0b00001010, 0x39, 0x00, 0x00], 0b10101010, 0b00001010); // Set accumulator to 0b10101010, LDy: 0x50, STA: 0x50, LDA: 0b00001010, AND 0x0000 y
    // and indirect X (Opcode: 0x21)
    test_and_or_instruction!(test_and_indirect_x, 10, [
        0xA2, 0x10,         // LDX #$10
        0xA9, 0x08,         // LDA #0x08
        0x85, 0x60,         // STA $60 (low byte of target address)
        0x85, 0x61,         // STA $61 (high byte of target address)
        0xA9, 0b10101010,   // LDA #0b10101010
        0x8D, 0x08, 0x08,   // STA $0808 (actual memory location operand)
        0xA9, 0b00001010,   // LDA #0b00001010 (value to AND with memory)
        0x21, 0x50          // AND ($50, X)
    ], 0b00001000, 0b00001010); // Expected: AND with 0b10101010 at $8008
    // and indirect Y (Opcode: 0x31)
    test_and_or_instruction!(test_and_indirect_y, 10, [  // Accum starts at 00
        0xA0, 0x10,         // LDY #$10 (Y = 0x10)
        0x85, 0x10,         // STA $10 (Low byte of target address)
        0xA9, 0x01,         // LDA #$01
        0x85, 0x11,         // STA $11 (High byte of target address)
        0xA9, 0b10101010,   // LDA #0b10101010
        0x8D, 0x10, 0x01,   // STA $0110 (target address = $0100 + Y)
        0xA9, 0b00001010,   // LDA #0b00001010
        0x31, 0x10          // AND ($10), Y -> AND value at ($10) + Y
    ], 0b00000000, 0b00001010);

    // OR instructions

    //or zero page (Opcode: 0x05)
    test_and_or_instruction!(test_or_zero_page, 3, [0x85, 0x50, 0xA9, 0b00001010, 0x05, 0x50], 0b10101010, 0b10101010); // Set accumulator to 0b10101010, STA: 0x50, LDA: 0b00001010, OR 0x50
    // or zero page x (Opcode: 0x15)
    test_and_or_instruction!(test_or_zero_page_x, 4, [0xa2, 0x50, 0x8d, 0x50, 0x00, 0xA9, 0b00001010, 0x15, 0x00], 0b10101010, 0b10101010); // Set accumulator to 0b10101010, LDX: 0x50, STA: 0x50, LDA: 0b00001010, OR 0x00 x
    // and abs (Opcode: 0x0D)
    test_and_or_instruction!(test_or_absolute, 3, [0xa2, 0x50, 0x00, 0xA9, 0b00001010, 0x0D, 0x50, 0x00], 0b10101010, 0b00001010); // Set accumulator to 0b10101010, STA: 0x50, LDA: 0b00001010, OR 0x0050
    // Or abs X (Opcode: 0x1D)
    test_and_or_instruction!(test_or_absolute_x, 4, [0xa2, 0x50, 0x8d, 0x50, 0x00, 0xA9, 0b00001010, 0x1D, 0x00, 0x00], 0b10101010, 0b10101010); // Set accumulator to 0b10101010, LDX: 0x50, STA: 0x50, LDA: 0b00001010, OR 0x0000 x
    // Or abs y (Opcode: 0x19)
    test_and_or_instruction!(test_or_absolute_y, 4, [0xa0, 0x50, 0x8d, 0x50, 0x00, 0xA9, 0b00001010, 0x19, 0x00, 0x00], 0b10101010, 0b10101010); // Set accumulator to 0b10101010, LDy: 0x50, STA: 0x50, LDA: 0b00001010, OR 0x0000 y
    // or indirect X (Opcode: 0x01)
    test_and_or_instruction!(test_or_indirect_x, 10, [
        0xA2, 0x10,         // LDX #$10
        0xA9, 0x08,         // LDA #0x08
        0x85, 0x60,         // STA $60 (low byte of target address)
        0x85, 0x61,         // STA $61 (high byte of target address)
        0xA9, 0b10101010,   // LDA #0b10101010
        0x8D, 0x08, 0x08,   // STA $0808 (actual memory location operand)
        0xA9, 0b00001010,   // LDA #0b00001010 (value to AND with memory)
        0x01, 0x50          // AND ($50, X)
    ], 0b00001000, 0b10101010);
    // or indirect Y (Opcode: 0x31)
    test_and_or_instruction!(test_or_indirect_y, 10, [  // Accum starts at 00
        0xA0, 0x10,         // LDY #$10 (Y = 0x10)
        0x85, 0x10,         // STA $10 (Low byte of target address)
        0xA9, 0x01,         // LDA #$01
        0x85, 0x11,         // STA $11 (High byte of target address)
        0xA9, 0b10101010,   // LDA #0b10101010
        0x8D, 0x10, 0x01,   // STA $0110 (target address = $0100 + Y)
        0xA9, 0b00001010,   // LDA #0b00001010
        0x11, 0x10          // AND ($10), Y -> AND value at ($10) + Y
    ], 0b00000000, 0b10101010);

}
