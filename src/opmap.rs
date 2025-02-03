use crate::cpu::CPU;

pub const OP_MAP: [fn(&mut CPU) -> (); 256] = {
    let mut map = [CPU::noop as fn(&mut CPU); 256];

    //'or' instructions
    map[0x09] = CPU::or_immediate;
    map[0x0d] = CPU::or_absolute;
    map[0x1d] = CPU::or_absolute_x;
    map[0x19] = CPU::or_absolute_y;
    map[0x05] = CPU::or_zero_page;
    map[0x15] = CPU::or_zero_page_x;
    map[0x01] = CPU::or_zero_page_x_indirect;
    map[0x11] = CPU::or_zero_page_y_indirect;

    //'store' instructions
    map[0x8d] = CPU::store_a_absolute;
    map[0x9d] = CPU::store_a_absolute_x;
    map[0x99] = CPU::store_a_absolute_y;
    map[0x85] = CPU::store_a_zero_page;
    map[0x95] = CPU::store_a_zero_page_x;
    map[0x81] = CPU::store_a_zero_page_x_indirect;
    map[0x91] = CPU::store_a_zero_page_y_indirect;

    //'store' X instructions
    map[0x8e] = CPU::store_x_absolute;
    map[0x86] = CPU::store_x_zero_page;
    map[0x96] = CPU::store_x_zero_page_y;

    //'store' to Y instructions
    map[0x8c] = CPU::store_y_absolute;
    map[0x84] = CPU::store_y_zero_page;
    map[0x94] = CPU::store_y_zero_page_x;

    map
};