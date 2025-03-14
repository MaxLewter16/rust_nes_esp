use rust_nes_esp::cpu::CPU;
use std::fs::File; // FOr testing NES File
use std::io::Write;
#[test]
fn test_nes_execution(){
    let mut cpu = match CPU::from_file_nestest(String::from("test_data/nes_test_data/nestest.nes")) {
        Ok(cpu) => cpu,
        Err(e) => {
            eprintln!("Failed to load NES file: {:?}", e);
            return;  // Exit the program
        }
    };
    cpu.execute_nestest(None, "test_data/nes_test_data/cpu_log.txt");
    println!("Test Result: {:02X}{:02X}", cpu.memory[0x0002], cpu.memory[0x0003])
}