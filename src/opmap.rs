use crate::cpu::CPU;

pub const OP_MAP: [fn(&mut CPU) -> (); 256] = {
    let mut map = [CPU::noop as fn(&mut CPU); 256];
    map[0x09] = CPU::or_immediate as fn(&mut CPU);
    map[0x0d] = CPU::or_absolute as fn(&mut CPU);
    map
};