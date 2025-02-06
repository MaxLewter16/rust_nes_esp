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

     //'and' instructions
    map[0x29] = CPU::and_immediate;
    map[0x2D] = CPU::and_absolute;
    map[0x3D] = CPU::and_absolute_x;
    map[0x39] = CPU::and_absolute_y;
    map[0x25] = CPU::and_zero_page;
    map[0x35] = CPU::and_zero_page_x;
    map[0x21] = CPU::and_zero_page_x_indirect;
    map[0x31] = CPU::and_zero_page_y_indirect;
    
    //'store' to A instructions
    map[0x8d] = CPU::store_a_absolute;
    map[0x9d] = CPU::store_a_absolute_x;
    map[0x99] = CPU::store_a_absolute_y;
    map[0x85] = CPU::store_a_zero_page;
    map[0x95] = CPU::store_a_zero_page_x;
    map[0x81] = CPU::store_a_zero_page_x_indirect;
    map[0x91] = CPU::store_a_zero_page_y_indirect;

    //'store' to X instructions
    map[0x8e] = CPU::store_x_absolute;
    map[0x86] = CPU::store_x_zero_page;
    map[0x96] = CPU::store_x_zero_page_y;

    //'store' to Y instructions
    map[0x8c] = CPU::store_y_absolute;
    map[0x84] = CPU::store_y_zero_page;
    map[0x94] = CPU::store_y_zero_page_x;

    //'transfer' instructions
    map[0xaa] = CPU::transfer_a_x;



    map
};