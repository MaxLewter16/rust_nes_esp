use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use rust_nes_esp::opmap::OP_NAME_MAP;

pub fn process_log_file(input_path: &str, output_path: &str) -> io::Result<()> {
    let input_file = File::open(input_path)?;   // Open the input file
    let reader = BufReader::new(input_file);    // Create a buffered reader
    let mut output_file = File::create(output_path)?;   // Create the output file

    for line in reader.lines() {
        let line = line?; // Read each line
        let parts: Vec<&str> = line.split([' ', ':']).collect(); // Split line into parts

        let address = parts[0];    // e.g., "C000"
        let opcode = parts[2];     // e.g., "4C"

        let mut a = "";
        let mut x = "";
        let mut y = "";
        let mut p = "";
        let mut sp = "";
        let mut cyc = "";

        for (i, part) in parts.iter().enumerate() {
            match *part {
                "A" => a = parts[i + 1],
                "X" => x = parts[i + 1],
                "Y" => y = parts[i + 1],
                "P" => p = parts[i + 1],
                "SP" => sp = parts[i + 1],
                "CYC" => cyc = parts[i + 1],
                _ => {}
            }
        }

        // Write the formatted line to the output file
        writeln!(output_file, "{:} OP:({:}){:30} A:{:} X:{:} Y:{:} P:{:} SP:{:} CYC:{:}", address, opcode, OP_NAME_MAP[u8::from_str_radix(opcode, 16).expect("") as usize], a, x, y, p, sp, cyc)?;
    }

    Ok(())
}


fn main() {
    let input_file = "test_data/nes_test_data/nestest.log";
    let output_file = "test_data/nes_test_data/nestest_log_processed_cyc.log";

    if let Err(e) = process_log_file(input_file, output_file) {
        eprintln!("Error processing file: {}", e);
    } else {
        println!("File processed successfully!");
    }
}
