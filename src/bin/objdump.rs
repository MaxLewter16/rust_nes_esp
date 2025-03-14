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
    num: Option<usize>,

    // Offset into ROM
    #[arg(short, long)]
    offset: Option<usize>,
}

fn obj_dump(obj_dump: ObjDump) -> Result<(), NesError> {
    let mem = Memory::from_file(obj_dump.file_path)?;
    let rom = mem.get_program_rom(obj_dump.program_id.unwrap_or(0));
    let offset = obj_dump.offset.unwrap_or(0);
    for (idx, instr) in rom[offset as u16..rom.len() as u16]
        .iter()
        .take(obj_dump.num.unwrap_or(usize::MAX))
        .enumerate()
    {
        let instr_name = if OP_NAME_MAP[*instr as usize] == "! INVALID !" {
            format!("INVALID - Value:0x{:x} Signed:{:}", *instr, *instr as i8)
        } else {
            String::from(OP_NAME_MAP[*instr as usize])
        };
        println!("0x{:<8x}:(0x{:2x}){:}", idx + offset, *instr, instr_name);
    }
    Ok(())
}

fn main() {
    if let Err(e) = obj_dump(ObjDump::parse()) {
        eprintln!("Error: {:?}", e);
    }
}
