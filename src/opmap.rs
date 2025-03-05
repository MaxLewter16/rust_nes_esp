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

    //'store' from A instructions
    map[0x8d] = CPU::store_a_absolute;
    map[0x9d] = CPU::store_a_absolute_x;
    map[0x99] = CPU::store_a_absolute_y;
    map[0x85] = CPU::store_a_zero_page;
    map[0x95] = CPU::store_a_zero_page_x;
    map[0x81] = CPU::store_a_zero_page_x_indirect;
    map[0x91] = CPU::store_a_zero_page_y_indirect;

    //'store' from X instructions
    map[0x8e] = CPU::store_x_absolute;
    map[0x86] = CPU::store_x_zero_page;
    map[0x96] = CPU::store_x_zero_page_y;

    //'store' from Y instructions
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

    //'branch' instructions
    map[0xb0] = CPU::branch_on_carry_set;
    map[0xf0] = CPU::branch_on_zero_set;
    map[0x30] = CPU::branch_on_negative_set;
    map[0x70] = CPU::branch_on_overflow_set;
    map[0x90] = CPU::branch_on_carry_reset;
    map[0xd0] = CPU::branch_on_zero_reset;
    map[0x10] = CPU::branch_on_negative_reset;
    map[0x50] = CPU::branch_on_overflow_reset;

    //flag control instructions
    map[0x38] = CPU::set_carry;
    map[0xf8] = CPU::set_decimal;
    map[0x78] = CPU::set_interrupt;
    map[0x18] = CPU::clear_carry;
    map[0xd8] = CPU::clear_decimal;
    map[0x58] = CPU::clear_interrupt;
    map[0xb8] = CPU::clear_overflow;

    //'adc' instructions
    map[0x69] = CPU::adc_immediate; //nice
    map[0x65] = CPU::adc_zero_page;
    map[0x75] = CPU::adc_zero_page_x;
    map[0x6D] = CPU::adc_absolute;
    map[0x7D] = CPU::adc_absolute_x;
    map[0x79] = CPU::adc_absolute_y;
    map[0x61] = CPU::adc_zero_page_x_indirect;
    map[0x71] = CPU::adc_zero_page_y_indirect;

    //'sbc' instructions
    map[0xE9] = CPU::sbc_immediate;
    map[0xE5] = CPU::sbc_zero_page;
    map[0xF5] = CPU::sbc_zero_page_x;
    map[0xED] = CPU::sbc_absolute;
    map[0xFD] = CPU::sbc_absolute_x;
    map[0xF9] = CPU::sbc_absolute_y;
    map[0xE1] = CPU::sbc_zero_page_x_indirect;
    map[0xF1] = CPU::sbc_zero_page_y_indirect;

    //'stack' instructions
    map[0x48] = CPU::push_a;
    map[0x08] = CPU::push_status;
    map[0x68] = CPU::pull_a;
    map[0x28] = CPU::pull_status;

    //'increment/decrement' instructions
    map[0xce] = CPU::dec_absolute;
    map[0xde] = CPU::dec_absolute_x;
    map[0xc6] = CPU::dec_zero_page;
    map[0xd6] = CPU::dec_zero_page_x;
    map[0xee] = CPU::inc_absolute;
    map[0xfe] = CPU::inc_absolute_x;
    map[0xe6] = CPU::inc_zero_page;
    map[0xf6] = CPU::inc_zero_page_x;
    map[0xca] = CPU::dec_x;
    map[0x88] = CPU::dec_y;
    map[0xe8] = CPU::inc_x;
    map[0xc8] = CPU::inc_y;

    // 'ctrl' instructions
    map[0x00] = CPU::break_instr;
    map[0x40] = CPU::return_from_interrupt;
    map[0x4c] = CPU::jump_absolute;
    map[0x6c] = CPU::jump_absolute_indirect;
    map[0x20] = CPU::jump_subroutine;
    map[0x60] = CPU::return_from_subroutine;

    // 'Arithmetic Shift Left' instructions
    map[0x0E] = CPU::asl_absolute;
    map[0x1E] = CPU::asl_absolute_x;
    map[0x06] = CPU::asl_zero_page;
    map[0x16] = CPU::asl_zero_page_x;
    map[0x0A] = CPU::asl_a;

    // 'Logical Shift Right' instructions
    map[0x4E] = CPU::lsr_absolute;
    map[0x5E] = CPU::lsr_absolute_x;
    map[0x46] = CPU::lsr_zero_page;
    map[0x56] = CPU::lsr_zero_page_x;
    map[0x4A] = CPU::lsr_a;

    // 'Rotate Right' instructions
    map[0x6E] = CPU::ror_absolute;
    map[0x7E] = CPU::ror_absolute_x;
    map[0x66] = CPU::ror_zero_page;
    map[0x76] = CPU::ror_zero_page_x;
    map[0x6A] = CPU::ror_a;

    // 'Rotate Left' instructions
    map[0x2E] = CPU::rol_absolute;
    map[0x3E] = CPU::rol_absolute_x;
    map[0x26] = CPU::rol_zero_page;
    map[0x36] = CPU::rol_zero_page_x;
    map[0x2A] = CPU::rol_a;


    map
};
