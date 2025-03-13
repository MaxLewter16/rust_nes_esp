
use crate::memory::{NesError, MMIO, RAM};
use bitflags::{bitflags, Flags};
use std::{cell::RefCell, u8};
#[cfg(feature = "image")]
use image::{GrayImage, RgbImage};

const VRAM_SIZE: u16 = 16 * (1 << 10);
const SPRAM_SIZE: u16 = 1 << 8;
const PATTERN_TABLE_SIZE: usize = 1 << 12;
const NAME_TABLE_SIZE: usize = 8 * 8 + 64;
const PALETTE: [[u8; 3]; 64] = [[0; 3]; 64];
const FRAME_WIDTH: usize = 256;
const FRAME_HEIGHT: usize = 240;

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

impl<'a> From<&'a [u8]> for PatternTable<'a> {
    fn from(value: &'a [u8]) -> Self {
        PatternTable { data: value.try_into().expect("") }
    }
}

struct NameTable<'a> {
    table_ids: &'a [u8],
    attribute: &'a [u8],
}

struct Attribute(u8);

impl<'a> From<&'a [u8]> for NameTable<'a> {
    fn from(value: &'a[u8]) -> Self {
        let (table_ids, attribute) = value.split_at(961);
        NameTable { table_ids, attribute}
    }
}

impl NameTable<'_> {

    // buf should be (32 * 8) * (30 * 8) * 3 = 184320 = 45*2^12 bytes
    // this is equivalent to a 256*240 RgbImage
    fn get_frame(&self, tables: &[PatternTable], buf: &mut[u8]) {


        //each chunk is one row of pixels in a pattern
        let mut table_row_pixels = buf.chunks_mut(8*3);
        for row in self.table_ids.chunks(32) {
            for row_pixels in 0..8 {
                for address in row.iter() {
                    let attribute_byte = self.attribute[((*address % 32) / 4 + (*address / 128) * 8) as usize];
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

    // write 8 pixels to the image buffer,
    // the pixels correspond to 'row' in the pattern at the 'address' in the given pattern table
    fn write_tile_row(&self, pattern_id: u8, row: usize, pattern: &PatternTable, buf: &mut[u8]) {
        // each attribute is split into 4 2-bit sections. Each section specifies the high color bits
        // of a 2x2 pattern grid
        let attribute_byte = self.attribute[((pattern_id % 32) / 4 + (pattern_id / 128) * 8) as usize];
        let shift_amnt = ((pattern_id % 4) / 2) | (((pattern_id / 32) % 2) << 1);

    }

    #[inline]
    fn map_pattern_to_attribute(&self, pattern: u8) -> u8 {
        // each attribute is split into 4 sections of 2-bits. Each section specifies the high color bits
        // of a 2x2 pattern grid.
        // every 4 columns of patterns is another attribute byte
        // every 4 rows of 32 patterns each is another row of attribute bytes
        let attribute_byte = self.attribute[((pattern % 32) / 4 + (pattern / 128) * 8) as usize];
        let shift_amnt = ((pattern % 4) / 2) | (((pattern / 32) % 2) << 1);
        (attribute_byte >> (3 - shift_amnt)) & 0x3
    }
}

#[derive(Debug, Clone, Copy)]
enum PPUState {
    PreRender(usize),
    VisibleLines(usize, PPUScanLineState),
    PostRender(usize),
    Vblank(usize),
}

#[derive(Debug, Clone, Copy)]
enum PPUScanLineState {
    Idle(usize),
    Render(usize),
    SpriteFetch(usize),
    PreFetch(usize),
    OtherFetch(usize),
}

// impl PPUState {
//     fn get_cycles(&self) -> usize {
//         let (PPUState::PreRender(cycles) | PPUState::VisibleLines(cycles) | PPUState::Vblank(cycles)) = self;
//         *cycles
//     }
// }

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
    state: PPUState,
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
            state: PPUState::PreRender(0),
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

    pub fn advance(&mut self, cycles: usize, buf: &mut [u8]) {
        const CYCLES_SCANLINE: usize = 341;
        const SCANLINES_VBLANK: usize = 20;
        const SCANLINES_VISIBLE: usize = 240;
        const SCANLINES_PRERENDER: usize = 1;
        const SCANLINES_POSTRENDER: usize = 1;
        const IDLE_CYCLES: usize = 1;
        const RENDER_CYCLES: usize = 256;
        const SPRITE_FETCH_CYCLES: usize = 64;
        const PRE_FETCH_CYCLES: usize = 16;
        const OTHER_FETCH_CYCLES: usize = 4;

        // ! TODO: odd cycle skip thing
        // ! TODO: sprite rendering
        // ! TODO: sprite hit detection
        match self.state {
            PPUState::PreRender(cycle) => {
                if cycle + cycles > SCANLINES_PRERENDER * CYCLES_SCANLINE {
                    self.state = PPUState::VisibleLines(
                        0,
                        PPUScanLineState::Idle(0));
                    self.advance(cycle + cycles - SCANLINES_PRERENDER * CYCLES_SCANLINE, buf);
                } else {
                    self.state = PPUState::PreRender(cycle + cycles);
                }
            },
            PPUState::VisibleLines(line, line_state) => {
                macro_rules! next_state {
                    ($current: expr, $threshhold: expr, $stay: path, $next: path) => {
                        if $current > $threshhold {
                            self.state = PPUState::VisibleLines(line, $next(0));
                            self.advance($current - $threshhold, buf);
                        } else {
                            self.state = PPUState::VisibleLines(line, $stay($current));
                        }
                    };
                }

                match line_state {
                    PPUScanLineState::Idle(cycle) => {
                        next_state!(cycle + cycles, IDLE_CYCLES, PPUScanLineState::Idle, PPUScanLineState::Render);
                    }
                    PPUScanLineState::Render(cycle) => {
                        let mut next = cycle / 8 * 8;
                        // rendering has granularity of 8 pixels, so every 8 ppu cycles
                        // 8 pixels are rendered. This is an approximation of hardware.
                        // this is to reduce memory accesses in software
                        let dest = (cycles + cycle) / 8 * 8;
                        while next < dest && next < RENDER_CYCLES {
                            let name_table_address = (self.ppu_control_1 & PPUControl1::NameTableAddressMask).bits() as u16 * NAME_TABLE_SIZE as u16 + 0x2000;

                            let pattern_idx = PPU::map_pixel_to_pattern(next);

                            let name_table: NameTable = self.vram[name_table_address..name_table_address + NAME_TABLE_SIZE as u16].into();

                            let pattern_address =
                                (((self.ppu_control_1 & PPUControl1::BackgroundTable).bits() as u16) << 8) &
                                ((name_table.table_ids[pattern_idx as usize] as u16) << 4);

                            let pattern: PatternTable = self.vram[pattern_address..pattern_address + 16].into();

                            pattern.write_rgb_row(
                                buf,
                                (next / FRAME_WIDTH) % 8,
                                name_table.map_pattern_to_attribute(pattern_idx) << 2 //TODO: high bits controlled by PPUControl2
                                );

                            next += 8;
                        }
                        next_state!(cycle + cycles, RENDER_CYCLES, PPUScanLineState::Render, PPUScanLineState::SpriteFetch);
                    }
                    PPUScanLineState::SpriteFetch(cycle) => {
                        next_state!(cycle + cycles, SPRITE_FETCH_CYCLES, PPUScanLineState::SpriteFetch, PPUScanLineState::OtherFetch);
                    }
                    PPUScanLineState::PreFetch(cycle) => {
                        next_state!(cycle + cycles, PRE_FETCH_CYCLES, PPUScanLineState::PreFetch, PPUScanLineState::OtherFetch);
                    }
                    PPUScanLineState::OtherFetch(cycle) => {
                        if cycle + cycles > OTHER_FETCH_CYCLES {
                            if line + 1 >= SCANLINES_VISIBLE {
                                self.state = PPUState::PostRender(0);
                            } else {
                                self.state = PPUState::VisibleLines(
                                    line + 1,
                                    PPUScanLineState::Idle(0));
                            }
                            self.advance(cycle + cycles - OTHER_FETCH_CYCLES, buf);
                        } else {
                            self.state = PPUState::VisibleLines(line, PPUScanLineState::OtherFetch(cycle + cycles));
                        }
                    }
                }
            },
            PPUState::PostRender(cycle) => {
                if cycle + cycles > SCANLINES_POSTRENDER * CYCLES_SCANLINE {
                    self.state = PPUState::Vblank(0);
                    self.advance(cycle + cycles - SCANLINES_POSTRENDER * CYCLES_SCANLINE, buf);
                } else {
                    self.state = PPUState::PostRender(cycle + cycles);
                }
            }
            PPUState::Vblank(cycle) => {
                let next = cycle + cycles;
                if cycle < 2 && next >= 2 {self.ppu_status |= PPUStatus::VBlankIndicator}
                if next > SCANLINES_VBLANK * CYCLES_SCANLINE {
                    self.state = PPUState::PreRender(0);
                    self.advance(next - SCANLINES_VBLANK * CYCLES_SCANLINE, buf);
                } else {
                    self.state = PPUState::Vblank(next);
                }
            }
        }
    }

    #[inline]
    const fn map_pixel_to_pattern(pixel: usize) -> u8 {
        // on average one pixel is rendered each cycle,
        // Each row of pattern tables corresponds to 8 rows of pixels in a frame, and pattern table has 8 columns of pixels
        ((pixel / (FRAME_WIDTH * 8))*8 + (pixel/FRAME_WIDTH) % 8) as u8
    }
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
