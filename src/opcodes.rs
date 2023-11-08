use crate::registers::Flags;

//https://robdor.com/2016/08/10/gameboy-emulator-half-carry-flag/ goddamn is that smart
pub fn half_carry_add(a: u8, b: u8) -> bool {
    (((a & 0xf) + (b & 0xf)) & 0x10) == 0x10
}
pub fn half_carry_sub(a: u8, b: u8) -> bool {
    (a & 0xf).wrapping_sub(b & 0xf) & 0x10 == 0x10
}
/// same as the half carries on u8's. focuses on the transfers
/// from bit 11 to bit 12.
pub fn half_carry_u16(a: u16, b: u16) -> bool {
    (((a & 0xFFF) + (b & 0xFFF)) & 0x1000) == 0x1000
}

/// these opcodes only modify the data provided and flags so 
/// no point in having them be stored in the struct / also avoids 
/// mutable borrowing errors
pub fn inc(data: &mut u8, flags: &mut Flags) {
    flags.set_h_flag(half_carry_add(*data, 1));
    *data = data.wrapping_add(1);
    flags.set_z_flag(*data == 0);
    flags.set_n_flag(false);
}
pub fn dec(data: &mut u8, flags: &mut Flags) {
    flags.set_h_flag(half_carry_sub(*data, 1));
    *data = data.wrapping_sub(1);
    flags.set_z_flag(*data == 0);
    flags.set_n_flag(true);
}
pub fn rr(reg: &mut u8, flags: &mut Flags) {
    // checks if a carry will occur in this shift
    let temp = *reg & 0b0000_0001;
    *reg = reg.rotate_right(1) | 0b1000_0000;
    *reg &= flags.c_flag() as u8;
    flags.set_c_flag(temp!=0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn rl(reg: &mut u8, flags: &mut Flags) {
    // checks if a carry will occur in this shift
    let carried = *reg & 0b1000_0000;
    flags.set_c_flag(carried>0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
    *reg = reg.rotate_left(1) | 0b0000_0001;
    *reg &= flags.c_flag() as u8;
}
pub fn rlc(reg: &mut u8, flags: &mut Flags) {
    *reg = reg.rotate_left(1);
    flags.set_c_flag((*reg&0b0000_0001)>0); // checks if 1st bit is set
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn rrc(reg: &mut u8, flags: &mut Flags) {
    *reg = reg.rotate_right(1);
    flags.set_c_flag((*reg&0b1000_0000)>0); // checks if 7th bit is set
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
pub fn sla(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg>=0b1000_0000);
    *reg = *reg << 1;
    flags.set_z_flag(*reg==0);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
}
pub fn sra(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg%2!=0); // meaning the 0th bit is set
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
pub fn bit(reg: &mut u8, index: u8, flags: &mut Flags) {
    flags.set_n_flag(false);
    flags.set_h_flag(true);
    let is_set = (*reg & (0b0000_0001 << index)) != 0;
    flags.set_z_flag(is_set);
}
pub fn res(reg: &mut u8, index: u8) {
    // reset the bit at the index
    *reg &= !(0b0000_0001 << index);
}
pub fn set(reg: &mut u8, index: u8) {
    *reg |= 0b0000_0001 << index;
}