
use crate::memory::RAM;

const VRAM_SIZE: u16 = 16 * (1 << 10);

struct PPU {
    vram: RAM,
}