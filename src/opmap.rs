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
    map[0x8a] = CPU::transfer_x_a;
    map[0xa8] = CPU::transfer_a_y;
    map[0x98] = CPU::transfer_y_a;
    map[0xba] = CPU::transfer_sp_x;
    map[0x9a] = CPU::transfer_x_sp;

    //'load' instructions
    map[0xa9] = CPU::load_a_immediate;
    map[0xad] = CPU::load_a_absolute;
    map[0xbd] = CPU::load_a_absolute_x;
    map[0xb9] = CPU::load_a_absolute_y;
    map[0xa5] = CPU::load_a_zero_page;
    map[0xb5] = CPU::load_a_zero_page_x;
    map[0xa1] = CPU::load_a_zero_page_x_indirect;
    map[0xb1] = CPU::load_a_zero_page_y_indirect;

    map[0xa2] = CPU::load_x_immediate;
    map[0xae] = CPU::load_x_absolute;
    map[0xbe] = CPU::load_x_absolute_y;
    map[0xa6] = CPU::load_x_zero_page;
    map[0xb6] = CPU::load_x_zero_page_y;

    map[0xa0] = CPU::load_y_immediate;
    map[0xac] = CPU::load_y_absolute;
    map[0xbc] = CPU::load_y_absolute_x;
    map[0xa4] = CPU::load_y_zero_page;
    map[0xb4] = CPU::load_y_zero_page_x;




    map
};