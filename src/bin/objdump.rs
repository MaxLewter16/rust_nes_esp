use rust_nes_esp::memory::{Memory, NesError};
use rust_nes_esp::opmap::OP_NAME_MAP;
use clap::Parser;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct ObjDump {
    // Path to .nes file
    file_path: String,

    // Program ROM to dump
    #[arg(short, long)]
    program_id: Option<usize>,

    // Number of instructions to display
    #[arg(short, long)]
    num: Option<usize>
}

fn obj_dump(obj_dump: ObjDump) -> Result<(), NesError> {
    let mem = Memory::from_file(obj_dump.file_path)?;
    for (idx, instr) in mem.get_program_rom(obj_dump.program_id.unwrap_or(0))
        .iter()
        .take(obj_dump.num.unwrap_or(usize::MAX))
        .enumerate()
    {
        println!("0x{:<8x}:{:}", idx * 8, OP_NAME_MAP[*instr as usize]);
    }
    Ok(())
}

fn main() {
    if let Err(e) = obj_dump(ObjDump::parse()) {
        eprintln!("Error: {:?}", e);
    }
}
