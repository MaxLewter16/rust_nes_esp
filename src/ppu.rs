
use crate::memory::{MMIO, RAM};
use bitflags::bitflags;

const VRAM_SIZE: u16 = 16 * (1 << 10);
const SPRAM_SIZE: u16 = 1 << 8;
const MMIO_WRITE_MAP: [fn(&mut PPU, u8); 8] = {
    let mut map = [PPU::ignore as fn(&mut PPU, u8); 8];
    map[0] = PPU::set_ppu_control_1;
    map[1] = PPU::set_ppu_control_2;
    map[3] = PPU::set_spr_ram_address;
    map[4] = PPU::write_spram;
    map[5] = PPU::set_scroll;
    map[6] = PPU::set_vram_address;
    map[7] = PPU::write_vram;
    map
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUControl1: u8 {
        const NameTableAddressMask = 0x03;
        const AddressIncrement = 0x04;
        const SpritePatternTable = 0x08;
        const BackgroundTable = 0x10;
        const SpriteSize = 0x20;
        const _MasterSlaveMode = 0x40;
        const IntteruptOnVBlank = 0x80;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUControl2: u8 {
        const ColorMode = 0x01;
        const BackgroundClip = 0x02;
        const SpriteClip = 0x04;
        const DisplayBackground = 0x08;
        const DisplaySprite = 0x10;
        const BackgroundColorMask = 0xe0;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PPUStatus: u8 {
        const VRAMWriteIndicator = 0x10;
        const ScanlineSpriteCount = 0x20;
        const SpriteCollision = 0x40;
        const VBlankIndicator = 0x80;
    }
}

pub struct PPU {
    vram: RAM,
    sprite_ram: RAM,
    ppu_control_1: PPUControl1,
    ppu_control_2: PPUControl2,
    ppu_status: PPUStatus,
    spr_ram_address: u8,
    vram_address: u16,
    byte_shift: u8,
    x_scroll: u8,
    y_scroll: u8
}

impl PPU {
    pub fn new() -> Self {
        PPU{
            vram: RAM::new::<{VRAM_SIZE as usize}>(0),
            sprite_ram: RAM::new::<{SPRAM_SIZE as usize}>(0),
            ppu_control_1: PPUControl1::from_bits_truncate(0),
            ppu_control_2: PPUControl2::from_bits_truncate(0),
            ppu_status: PPUStatus::from_bits_truncate(0),
            spr_ram_address: 0,
            vram_address: 0,
            byte_shift: 8,
            x_scroll: 0,
            y_scroll: 0,
        }
    }

    pub fn write(&mut self, address: u16, data: u8) {
        if let Some(address) = address.checked_sub(0x2000) {
            if let Some(write) = MMIO_WRITE_MAP.get(address as usize) {
                write(self, data);
            }
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        match address {
            0x2002 => {
                self.byte_shift = 8;
                self.ppu_status.bits()
            },
            0x2004 => self.sprite_ram[address],
            0x2007 => self.vram[address],
            _ => 0,
        }
    }

    pub fn set_ppu_control_1(&mut self, data: u8) {
        self.ppu_control_1 = PPUControl1::from_bits_retain(data);
    }

    pub fn set_ppu_control_2(&mut self, data: u8) {
        self.ppu_control_2 = PPUControl2::from_bits_retain(data);
    }

    pub fn set_spr_ram_address(&mut self, data: u8) {
        self.spr_ram_address = data;
    }

    pub fn set_scroll(&mut self, data: u8) {
        if self.byte_shift != 0 {
            self.x_scroll = data;
        } else {
            self.y_scroll = data;
        }
        if self.byte_shift == 0 {self.byte_shift = 8} else {self.byte_shift = 0}
    }

    pub fn set_vram_address(&mut self, data: u8) {
        //clear bits to write
        self.vram_address &= !(0xff << self.byte_shift);
        //write address portion, ignore upper two bits
        self.vram_address |= ((data as u16) << self.byte_shift) & 0xa0;
        if self.byte_shift == 0 {self.byte_shift = 8} else {self.byte_shift = 0}
        if self.ppu_control_1.contains(PPUControl1::AddressIncrement) {self.vram_address += 32} else {self.vram_address += 1}
    }

    pub fn write_spram(&mut self, data: u8) {
        self.sprite_ram[self.spr_ram_address as u16] = data;
    }

    pub fn write_vram(&mut self, data: u8) {
        self.vram[self.vram_address % VRAM_SIZE] = data;
    }

    pub fn ignore(&mut self, _data: u8) {}
}