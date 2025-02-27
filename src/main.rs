#![allow(unused_variables)]

pub mod cpu;
pub mod opmap;
pub mod ppu;
pub mod memory;
use crate::opmap::OP_MAP;
use crate::cpu::CPU;

fn count_valid_ops() -> usize {
    OP_MAP.iter().filter(|&&op| op as usize != CPU::noop as usize).count()
}

fn main() {
    let valid_opcodes = count_valid_ops();
    println!("Number of implemented opcodes: {}", valid_opcodes);
}

