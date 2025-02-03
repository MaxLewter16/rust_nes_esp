use crate::cpu::CPU;

pub const OP_MAP: [fn(&mut CPU) -> (); 256] = {
    let mut map = [CPU::noop as fn(&mut CPU); 256];

    //'or' instructions
    map[0x09] = CPU::or_immediate as fn(&mut CPU);
    map[0x0d] = CPU::or_absolute as fn(&mut CPU);
    map[0x1d] = CPU::or_absolute_x as fn(&mut CPU);
    map[0x19] = CPU::or_absolute_y as fn(&mut CPU);
    map[0x05] = CPU::or_zero_page as fn(&mut CPU);
    map[0x15] = CPU::or_zero_page_x as fn(&mut CPU);
    map[0x01] = CPU::or_zero_page_x_indirect as fn(&mut CPU);
    map[0x11] = CPU::or_zero_page_y_indirect as fn(&mut CPU);

    //'store' instructions
    map[0x8d] = CPU::store_a_absolute as fn(&mut CPU);

    map
};