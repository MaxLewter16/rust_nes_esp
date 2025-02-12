
use crate::memory::RAM;

const VRAM_SIZE: u16 = 16 * (1 << 10);

struct PPU {
    vram: RAM,
}

impl PPU {
    pub fn new() -> Self {
        PPU{vram: RAM::new::<{VRAM_SIZE as usize}>(0)}
    }
}