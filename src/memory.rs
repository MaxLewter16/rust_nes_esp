use std::{io::{self, Read}, marker::PhantomPinned, ops::{Bound, Deref, DerefMut, Index, IndexMut, Range}, ptr::NonNull, u16};
use std::result::Result;
use crate::ppu::PPU;

// Memory Map constants
// constants specify the start of named section
pub const BUILTIN_RAM: u16 = 0;
pub const MMIO: u16 = 0x2000;
pub const EXPANSION_ROM: u16 = 0x4020;
pub const SRAM: u16 = 0x6000;
pub const PROGRAM_ROM: u16 = 0x8000;
pub const PROGRAM_ROM_SIZE: u16 = 16 * (1 << 10);
pub const PROGRAM_ROM_2: u16 = PROGRAM_ROM + PROGRAM_ROM_SIZE;
pub const VROM_SIZE: u16 = 0x1000;
pub const BATTERY_RAM: u16 = 0x6000;
pub const BATTERY_RAM_SIZE: u16 = 0x2000;
pub const TRAINER_SIZE: u16 = 1 << 9;

const MMIO_WRITE_MAP: [fn(&mut PPU, u8); 8] = {
    let mut map = [PPU::ignore as fn(&mut PPU, u8); 8];
    //MMIO addresses [0x2000,0x2008)
    map[0] = PPU::set_ppu_control_1;
    map[1] = PPU::set_ppu_control_2;
    map[3] = PPU::set_spr_ram_address;
    map[4] = PPU::write_spram;
    map[5] = PPU::set_scroll;
    map[6] = PPU::set_vram_address;
    map[7] = PPU::write_vram;
    //MMIO addresses starting [0x4000,0x4020):
    map
};

//map an address to an index in 'MMIO_WRITE_MAP' which handles mmio at the address
const fn address_mmio_map(address: u16) -> usize {
    match address {
        0x2000..0x4000 => (address as usize - 0x2000) % 0x8,
        0x4000..0x4020 => address as usize - 0x4000,
        _ => panic!("invalid mmio address")
    }
}

pub struct RAM {
    file: Box<[u8]>,
    start_address: u16,
}

impl RAM {
    pub fn new<const S: usize>(start: u16) -> Self {
        Self{file: Box::new([0u8; S]), start_address: start}
    }

    /// Return Some(RAM) if space can be allocated, otherwise None.
    /// Return None if size is 0
    pub fn new_dyn(size: usize, start: u16) -> Option<Self> {
        if size == 0 {return None}
        // this unsafe block does the equivalent of Box::new_zeroed_slice().assume_init()
        /*
            This is safe because:
                - The box allocator and the allocator used to allocate the slice match
                - The u8 primitive type has an allignment of 1, and [u8] has the same allignment
                - '0' is a valid value for integer types
                - size is non-zero
         */
        let zeroed_mem = unsafe {
            let slice_alloc = std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(size_of::<u8>() * size, 1).expect(""));
            if slice_alloc.is_null() {return None}
            Box::from_raw(std::slice::from_raw_parts_mut(slice_alloc,size) as *mut [u8])
        };
        Some(Self{file: zeroed_mem, start_address: start})
    }

    // *Note: Deref<Target = [u8]> is not implemented because indexing is different
    // *from a typical slice
    pub fn as_slice(&self) -> &[u8] {
        &self.file
    }

    pub fn as_slice_mut(&mut self) -> &mut[u8] {
        &mut self.file
    }
}

impl Index<u16> for RAM {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        &self.file[(address - self.start_address) as usize]
    }
}

impl IndexMut<u16> for RAM {
    fn index_mut(&mut self, address: u16) -> &mut Self::Output {
        &mut self.file[(address - self.start_address) as usize]
    }
}

impl Index<Range<u16> > for RAM {
    type Output = [u8];

    fn index(&self, index: Range<u16>) -> &Self::Output {
        &self.file[(index.start - self.start_address) as usize .. (index.end - self.start_address) as usize]
    }
}

enum ProgramROMDst {
    One,
    Two
}

#[derive(Debug)]
pub enum NesError {
    IO(io::Error),
    FileFormat(&'static str),
    Emulator(&'static str)
}

impl From<io::Error> for NesError {
    fn from(value: io::Error) -> Self {
        NesError::IO(value)
    }
}

pub struct Memory {
    program_rom: Vec<RAM>,
    /* Memory must uphold the following:
        - active_program_1/2 must be non-null
        - active_program_1/2 should not be used to modify program memory
       Because reading program rom occurs every emulated cycle it should have
       minimal overhead, which is achieved with a pointer to the active memory.
    */
    active_program_1: NonNull<RAM>,
    active_program_2: NonNull<RAM>,
    // because Memory contains pointers to itself it can't be moved
    _phantom_pin: PhantomPinned,
    ram: [u8; (MMIO - BUILTIN_RAM) as usize],
    battery_ram: Option<RAM>,
    pub ppu: PPU,
    mapper: u8, //TODO should be enum probably
}

impl Index<u16> for Memory {
    type Output = u8;
    fn index(&self, address: u16) -> &Self::Output {
        match address {
            BUILTIN_RAM..MMIO => &self.ram[(address % 0x0800) as usize], // Mirror every 2 KB
            MMIO..EXPANSION_ROM => self.ppu.read(address), // Mirrors every 8 bytes
            EXPANSION_ROM..SRAM => &0u8, //EXPANSION_ROM
            SRAM..PROGRAM_ROM => if let Some(ref ram) = self.battery_ram {
                &ram[address]
            } else {
                // ! What should these reads return
                &0u8
            },
            // this is safe because active program roms are always selected
            PROGRAM_ROM..PROGRAM_ROM_2 => unsafe{&self.active_program_1.as_ref()[address]},
            PROGRAM_ROM_2..=u16::MAX => unsafe{&self.active_program_2.as_ref()[address]},
        }
    }
}

impl Memory {
    pub fn write(&mut self, address: u16, data: u8) {
        match address {
            BUILTIN_RAM..MMIO => self.ram[(address % 0x0800) as usize] = data, // Mirror every 2 KB
            MMIO..EXPANSION_ROM => MMIO_WRITE_MAP[address_mmio_map(address)](&mut self.ppu, data),
            EXPANSION_ROM..SRAM => (), //EXPANSION_ROM
            SRAM..PROGRAM_ROM => if let Some(ref mut ram) = self.battery_ram {
                ram[address] = data;
            } , // SRAM (not yet implemented)
            // TODO: writes to program rom are used to control memory mappers
            PROGRAM_ROM..PROGRAM_ROM_2 => (),
            PROGRAM_ROM_2..=u16::MAX => (),
        }
    }

    pub fn set_active_ram(&mut self, src: usize, dst: ProgramROMDst) {
        match dst {
            ProgramROMDst::One => {
                self.program_rom[src].start_address = PROGRAM_ROM;
                self.active_program_1 = NonNull::new(&mut self.program_rom[src]).unwrap();
            }
            ProgramROMDst::Two => {
                self.program_rom[src].start_address = PROGRAM_ROM_2;
                self.active_program_2 = NonNull::new(&mut self.program_rom[src]).unwrap();
            }
        }
    }

    pub fn from_program(mut program: Vec<u8>) -> Self {
        program.resize(0x10000 - PROGRAM_ROM as usize, 0);
        let mut program = RAM{file: program.into_boxed_slice(),start_address: PROGRAM_ROM};
        let ap1 = NonNull::new(&mut program).unwrap();
        let ap2 = NonNull::new(&mut program).unwrap();
        Memory {
            program_rom: vec![program],
            active_program_1: ap1,
            active_program_2: ap2,
            ram: [0u8; (MMIO - BUILTIN_RAM) as usize],
            battery_ram: None,
            mapper: 0,
            ppu: PPU::new(vec![]),
            _phantom_pin: PhantomPinned
        }
    }

    pub fn from_file(path: String) -> Result<Self, NesError> {
        let mut file = std::fs::File::open(path)?;
        let mut header = [0u8; 16];
        if file.read(&mut header)? < 16 {return Err(NesError::FileFormat("file too short"))};
        if header[0..4] != ['N' as u8, 'E' as u8, 'S' as u8, 0x1a] {
            return Err(NesError::FileFormat("incorrect identifying bytes, not a .nes file?"))
        };

        if (header[7] & 0x0c) == 0x08 {eprintln!("Warning: NES2.0 file format unsupported")}

        let prg_rom_count = header[4];
        let vrom_count = header[5];
        let rom_control = &header[6..8];
        let ram_bank_count = header[8];

        let mapper_number = (rom_control[1] & 0xf0) | (rom_control[0] >> 4);
        let mirroring_type = (rom_control[0] & 1) != 0;
        let battery_ram = (rom_control[0] & 2) != 0;
        let trainer = (rom_control[0] & 4) != 0;
        if !battery_ram && trainer {panic!("idx what happens in this case");}

        let battery_ram = if battery_ram {
            let mut ram = Box::new([0u8; BATTERY_RAM_SIZE as usize]);
            if trainer {
                file.read( &mut ram.as_mut_slice()[0x1000..0x1200])?;
            }
            Some(RAM{file: ram, start_address: BATTERY_RAM})
        } else {
            None
        };

        let mut program = Vec::new();
        let mut vrom = Vec::new();

        for _ in 0..prg_rom_count {
            let mut prg_rom_buf = Box::new([0u8; PROGRAM_ROM_SIZE as usize]);
            file.read_exact(prg_rom_buf.as_mut_slice())?;
            program.push(RAM{file: prg_rom_buf, start_address: PROGRAM_ROM})
        }

        for _ in 0..vrom_count {
            let mut vrom_buf = Box::new([0u8; VROM_SIZE as usize]);
            file.read_exact(vrom_buf.as_mut_slice())?;
            vrom.push(RAM{file: vrom_buf, start_address: 0})
        }

        // by default load a single program rom which is mirrored
        let active_program_1 = NonNull::new(&mut program[0]).unwrap();
        let active_program_2 = NonNull::new(&mut program[0]).unwrap();

        Ok(Memory{
            program_rom: program,
            active_program_1,
            active_program_2,
            ram: [0u8; (MMIO - BUILTIN_RAM) as usize],
            battery_ram: battery_ram,
            mapper: mapper_number,
            ppu: PPU::new(vrom),
            _phantom_pin: PhantomPinned
        })
    }

    fn mmio(&self, address: u16) -> &u8 {
        //TODO
        // MMIO_MAP[address]();
        unimplemented!()
    }
}

