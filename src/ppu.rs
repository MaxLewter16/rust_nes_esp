
use crate::memory::{MMIO, RAM};
use bitflags::{bitflags, Flags};
use std::{cell::RefCell, u8};
#[cfg(feature = "image")]
use image::{GrayImage, RgbImage};

const VRAM_SIZE: u16 = 16 * (1 << 10);
const SPRAM_SIZE: u16 = 1 << 8;
const PATTERN_TABLE_SIZE: usize = 1 << 12;
const PALETTE: [[u8; 3]; 64] = [[0; 3]; 64];

struct PatternTable<'a> {
    data: &'a [u8; 16],
}

impl PatternTable<'_> {
    // Returns a value between 0-3 for an 8x8 grid of pixels
    fn get_pixel(&self, idx: (usize, usize)) -> u8 {
        let (i, j) = idx;
        // Getting the 7-j bit of the ith data
        let low_bit = (self.data[i] >> (7 - j)) & 1;
        // Getting the ith + 8 data shifted 7-j bits
        let high_bit = (self.data[i + 8] >> (7 - j)) & 1;
        low_bit | ( high_bit <<  1 )
    }

    // writes pixels where pixels[0][0] is the upper left and pixels[15][15] is bottom right
    // *NOTE: scales pixel value for a greyscale image
    fn write_greyscale_pixels(&self, pixels: &mut[[u8; 8]]) {
        for i in 0..8 {
            for j in 0..8 {
                pixels[i][j] = self.get_pixel((i,j)) << 7;
            }
        }
    }

    fn write_rgb_row(&self, pixels: &mut[u8], row: usize, upper_bits: u8) {
        let mut pixels_view = pixels.chunks_mut(3);
        for j in 0..8 {
            pixels_view.next().unwrap().copy_from_slice(&PALETTE[(upper_bits | self.get_pixel((row,j))) as usize]);
        }
    }

}

#[cfg(feature = "image")]
impl PatternTable<'_> {
    fn generate_pattern_table_image(pattern_tables: &[u8; PATTERN_TABLE_SIZE as usize]) -> GrayImage {
        let mut image = Vec::new();
        image.resize(1 << 14, 0u8);
        let mut image_view: Vec<&mut [u8]> = image.chunks_mut(8).collect();
        let mut pixel_tmp = [[0u8; 8]; 8];
        for (id, pattern_table) in pattern_tables.chunks(16).map(|s| PatternTable{data: s.try_into().expect("")}).enumerate(){
            pattern_table.write_greyscale_pixels(&mut pixel_tmp);
            // 16 tiles per row
            // 8 rows per tile layer
            for row in 0..8 {image_view[(id/16)*128 + id%16 + row*16].copy_from_slice(&pixel_tmp[row])}
        }
        GrayImage::from_vec(1 << 7, 1 << 7, image).unwrap()
    }
}

struct NameTable<'a> {
    table_ids: &'a [u8; 960],
    attribute: &'a [Attribute; 64],
}

struct Attribute(u8);


impl NameTable<'_> {

    // buf should be (32 * 8) * (30 * 8) * 3 = 184320 = 45*2^12 bytes
    // this is equivalent to a 256*240 RgbImage
    fn get_frame(&self, tables: &[PatternTable], buf: &mut[u8]) {
        const FRAME_WIDTH: usize = 256;
        const FRAME_HEIGHT: usize = 240;

        //each chunk is one row of pixels in a pattern
        let mut table_row_pixels = buf.chunks_mut(8*3);
        for row in self.table_ids.chunks(32) {
            for row_pixels in 0..8 {
                for address in row.iter() {
                    let attribute_byte = self.attribute[((*address % 32) / 4 + (*address / 128) * 8) as usize].0;
                    let shift_amnt = ((*address % 4) / 2) | (((*address / 32) % 2) << 1);
                    tables[*address as usize].write_rgb_row(
                        table_row_pixels.next().unwrap(),
                        row_pixels,
                        (attribute_byte >> (3 - shift_amnt) & 0x3) << 2
                        );
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PPUStatus(u8);

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

    impl PPUStatus: u8 {
        const VRAMWriteIndicator = 0x10;
        const ScanlineSpriteCount = 0x20;
        const SpriteCollision = 0x40;
        const VBlankIndicator = 0x80;
    }
}

pub struct PPU {
    vrom: Vec<RAM>,
    vram: RAM,
    sprite_ram: RAM,
    ppu_control_1: PPUControl1,
    ppu_control_2: PPUControl2,
    ppu_status: PPUStatus,
    spr_ram_address: u8,
    vram_address: u16,
    // Allow mutability on reads of PPU through non-mutable reference.
    // This is purely interior state, a reference to this data should never shared.
    // Modifying this data through a non-mutable reference won't panic as long
    // as a reference to this data is never shared.
    byte_shift: RefCell<u8>,
    x_scroll: u8,
    y_scroll: u8
}

impl PPU {
    pub fn new(vrom: Vec<RAM>) -> Self {
        let mut ppu = PPU{
            vram: RAM::new::<{VRAM_SIZE as usize}>(0),
            vrom,
            sprite_ram: RAM::new::<{SPRAM_SIZE as usize}>(0),
            ppu_control_1: PPUControl1::from_bits_truncate(0),
            ppu_control_2: PPUControl2::from_bits_truncate(0),
            ppu_status: PPUStatus::from_bits_truncate(0),
            spr_ram_address: 0,
            vram_address: 0,
            byte_shift: 8.into(),
            x_scroll: 0,
            y_scroll: 0,
        };

        if ppu.vrom.len() > 0 {
            // by default load first two vroms into program tables
            // if only a single vrom is present, duplicate this vrom
            ppu.load_vrom(0, 0);
            ppu.load_vrom(if ppu.vrom.len() > 1 {1} else {0}, 1);
        }

        ppu
    }

    /*
        dst: 1 or 0, target pattern table
        src: vrom to load
     */
    pub fn load_vrom(&mut self, src: usize, dst: usize) {
        self.vram.as_slice_mut()[dst*PATTERN_TABLE_SIZE..(dst+1)*PATTERN_TABLE_SIZE].copy_from_slice(self.vrom[src].as_slice());
    }

    pub fn read(&self, address: u16) -> &u8 {
        match address {
            0x2002 => {
                self.byte_shift.replace(8);
                &self.ppu_status.0
            }
            0x2004 => &self.sprite_ram[address],
            0x2007 => &self.vram[address],
            _ => &0,
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
        if *self.byte_shift.borrow() != 0 {
            self.x_scroll = data;
        } else {
            self.y_scroll = data;
        }
        if *self.byte_shift.borrow() == 0 {self.byte_shift.replace(8);} else {self.byte_shift.replace(0);}
    }

    pub fn set_vram_address(&mut self, data: u8) {
        //clear bits to write
        self.vram_address &= !(0xff << *self.byte_shift.borrow());
        //write address portion, ignore upper two bits
        self.vram_address |= ((data as u16) << *self.byte_shift.borrow()) & 0xa0;
        if *self.byte_shift.borrow() == 0 {self.byte_shift.replace(8);} else {self.byte_shift.replace(0);}
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

mod tests {
    use super::*;
    use crate::cpu::CPU;
    use crate::memory::Memory;

    #[cfg(feature = "image")]
    #[test]
    fn test_pattern_table_image() {
        let mem = Memory::from_file(String::from("../galaga.nes")).expect("failed to load file");
        for (i, table) in mem.ppu.vrom.iter().enumerate() {
            let image= PatternTable::generate_pattern_table_image(table.as_slice().try_into().expect("incorrectly sized pattern table"));
            image.save_with_format(format!("pattern_table_{i}.png"), image::ImageFormat::Png).expect("failed to save pattern table to png");
        }
    }
}
