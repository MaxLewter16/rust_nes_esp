use rust_nes_esp::cpu::CPU;

#[test]
fn test_nes_execution(){
    let mut cpu = match CPU::from_file_nestest(String::from("test_data/nes_test_data/nestest.nes")) {
        Ok(cpu) => cpu,
        Err(e) => {
            eprintln!("Failed to load NES file: {:?}", e);
            return;  // Exit the program
        }
    };
    // Test all instructions, undocumented instructions start at ~5000 seemingly
    cpu.execute_with_logging(Some(8991), "test_data/nes_test_data/cpu_log.txt");
    println!("Test Result: 0x{:02X}: 0x{:02X}", cpu.memory.read(0x0002), cpu.memory.read(0x0003));
    assert!(cpu.memory.read(0x0002) == 0);
}