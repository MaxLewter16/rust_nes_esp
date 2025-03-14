#![allow(unused_variables)]
pub mod cpu;
pub mod opmap;
pub mod ppu;
pub mod memory;
mod nestest_log_processor;
use crate::opmap::OP_MAP;
use crate::cpu::CPU;

fn count_valid_ops() -> usize {
    OP_MAP.iter().filter(|&&op| op as usize != CPU::noop as usize).count()
}

fn main() {
    let valid_opcodes = count_valid_ops();
    println!("Number of implemented opcodes: {}", valid_opcodes);
    let input_file = "test_data/nes_test_data/nestest.log";
    let output_file = "test_data/nes_test_data/nestest_log_processed.log";

    if let Err(e) = nestest_log_processor::process_log_file(input_file, output_file) {
        eprintln!("Error processing file: {}", e);
    } else {
        println!("File processed successfully!");
    }

    }

