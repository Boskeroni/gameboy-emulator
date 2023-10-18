use crate::registers::Flags;

//https://robdor.com/2016/08/10/gameboy-emulator-half-carry-flag/ goddamn is that smart
pub fn half_carry_u8_add(a: u8, b: u8) -> bool {
    (((a & 0xf) + (b & 0xf)) & 0x10) == 0x10
}
pub fn half_carry_u8_sub(a: u8, b: u8) -> bool {
    (((a & 0xf).wrapping_sub(b & 0xf)) & 0x10) == 0x10
}
/// values 0x7FF and 0x800 were calculated with same method from previous
pub fn half_carry_u16(a: u16, b: u16) -> bool {
    (((a & 0x7FF) + (b & 0x7FF)) & 0x800) == 0x800
}

/// these opcodes only modify the data provided and flags so 
/// no point in having them be stored in the struct / also avoids 
/// mutable borrowing errors
pub fn add_u16(reg1: u16, reg2: u16, flags: &mut Flags) -> u16 {
    print!("add {reg1} and {reg2}");
    let half_carried = half_carry_u16(reg1, reg2);
    flags.set_h_flag(half_carried);
    let sum = reg1 + reg2;
    flags.set_c_flag(sum<reg1);
    flags.set_n_flag(false);
    sum
}
pub fn inc(data: &mut u8, flags: &mut Flags) {
    print!("inc {data}");
    let half_carried = half_carry_u8_add(*data, 1); 
    flags.set_h_flag(half_carried);
    *data = data.wrapping_add(1);
    flags.set_z_flag(*data == 0);
    flags.set_n_flag(false);
}
pub fn dec(data: &mut u8, flags: &mut Flags) {
    print!("dec {data}");
    flags.set_h_flag(half_carry_u8_sub(*data, 1));
    *data -= 1;
    flags.set_z_flag(*data == 0);
    flags.set_n_flag(true);
}
pub fn rr(reg: &mut u8, flags: &mut Flags) {
    print!("right rotate {reg}");
    // checks if a carry will occur in this shift
    let temp = *reg & 0b0000_0001;
    *reg = reg.rotate_right(1) ^ ((flags.get_c_flag() as u8)<<7);
    flags.set_c_flag(temp!=0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn rl(reg: &mut u8, flags: &mut Flags){
    print!("left rotate {reg}");
    // checks if a carry will occur in this shift
    let carried = *reg & 0b1000_0000;
    flags.set_c_flag(carried>0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
    *reg = reg.rotate_left(1) ^ (flags.get_c_flag() as u8)
}
pub fn rlc(reg: &mut u8, flags: &mut Flags) {
    *reg = reg.rotate_left(1);
    flags.set_c_flag(*reg&0b0000_0001%2 == 0); // checks if 1st bit is set
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn rrc(reg: &mut u8, flags: &mut Flags) {
    *reg = reg.rotate_right(1);
    flags.set_c_flag(*reg&0b1000_0000>0); // checks if 7th bit is set
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn sla(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg>0b1000_0000);
    *reg = *reg << 1;
    flags.set_z_flag(*reg==0);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
}
pub fn sra(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg%2==1); // meaning the 0th bit is set
    *reg = (*reg >> 1) + (*reg & 0b1000_0000);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn swap(reg: &mut u8, flags: &mut Flags) {
    let temp = *reg & 0b0000_1111;
    *reg = (*reg >> 4) + (temp << 4);
    flags.set_z_flag(*reg==0);
    flags.set_c_flag(false);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
}
pub fn srl(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg%2==1); // meaning the 0th bit is set
    *reg = *reg >> 1;
    flags.set_z_flag(*reg==0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
}
pub fn bit(index: u8, reg: &mut u8, flags: &mut Flags) {
    flags.set_n_flag(false);
    flags.set_h_flag(true);
    let bit_index = 0b0000_0001 << index;
    let is_set = (*reg & bit_index) != 0;
    flags.set_z_flag(is_set);
}
pub fn res(index: u8, reg: &mut u8) {
    *reg &= !(0b0000_0001 << index);
}
pub fn set(index: u8, reg: &mut u8) {
    *reg |= 0b0000_0001 << index;
}