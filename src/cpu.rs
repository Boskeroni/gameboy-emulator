use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;
use crate::{registers::*, combine_u8s};

//https://robdor.com/2016/08/10/gameboy-emulator-half-carry-flag/ goddamn is that smart
fn half_carry_u8_add(a: u8, b: u8) -> bool {
    (((a & 0xf) + (b & 0xf)) & 0x10) == 0x10
}
fn half_carry_u8_sub(a: u8, b: u8) -> bool {
    (((a & 0xf).wrapping_sub(b & 0xf)) & 0x10) == 0x10
}
/// values 0x7FF and 0x800 were calculated with same method from previous
fn half_carry_u16(a: u16, b: u16) -> bool {
    (((a & 0x7FF) + (b & 0x7FF)) & 0x800) == 0x800
}

/// these opcodes only modify the data provided and flags so 
/// no point in having them be stored in the struct / also avoids 
/// mutable borrowing errors
fn add_u16(reg1: u16, reg2: u16, flags: &mut Flags) -> u16 {
    print!("add {reg1} and {reg2}");
    let half_carried = half_carry_u16(reg1, reg2);
    flags.set_h_flag(half_carried);
    let sum = reg1 + reg2;
    flags.set_c_flag(sum<reg1);
    flags.set_n_flag(false);
    sum
}
fn inc(data: &mut u8, flags: &mut Flags) {
    print!("inc {data}");
    let half_carried = half_carry_u8_add(*data, 1); 
    flags.set_h_flag(half_carried);
    *data += 1;
    flags.set_z_flag(*data == 0);
    flags.set_n_flag(false);
}
fn dec(data: &mut u8, flags: &mut Flags) {
    print!("dec {data}");
    flags.set_h_flag(half_carry_u8_sub(*data, 1));
    *data -= 1;
    flags.set_z_flag(*data == 0);
    flags.set_n_flag(true);
}
fn rr(reg: &mut u8, flags: &mut Flags) {
    print!("right rotate {reg}");
    // checks if a carry will occur in this shift
    let temp = *reg & 0b0000_0001;
    *reg = reg.rotate_right(1) ^ ((flags.get_c_flag() as u8)<<7);
    flags.set_c_flag(temp!=0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
    flags.set_z_flag(*reg==0);
}
fn rl(reg: &mut u8, flags: &mut Flags){
    print!("left rotate {reg}");
    // checks if a carry will occur in this shift
    let carried = *reg & 0b1000_0000;
    flags.set_c_flag(carried>0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
    *reg = reg.rotate_left(1) ^ (flags.get_c_flag() as u8)
}
fn rlc(reg: &mut u8, flags: &mut Flags) {
    *reg = reg.rotate_left(1);
    flags.set_c_flag(*reg&0b0000_0001%2 == 0); // checks if 1st bit is set
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
fn rrc(reg: &mut u8, flags: &mut Flags) {
    *reg = reg.rotate_right(1);
    flags.set_c_flag(*reg&0b1000_0000>0); // checks if 7th bit is set
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
fn sla(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg>0b1000_0000);
    *reg = *reg << 1;
    flags.set_z_flag(*reg==0);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
}
fn sra(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg%2==1); // meaning the 0th bit is set
    *reg = (*reg >> 1) + (*reg & 0b1000_0000);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
    flags.set_z_flag(*reg==0);
}
fn swap(reg: &mut u8, flags: &mut Flags) {
    let temp = *reg & 0b0000_1111;
    *reg = (*reg >> 4) + (temp << 4);
    flags.set_z_flag(*reg==0);
    flags.set_c_flag(false);
    flags.set_h_flag(false);
    flags.set_n_flag(false);
}
fn srl(reg: &mut u8, flags: &mut Flags) {
    flags.set_c_flag(*reg%2==1); // meaning the 0th bit is set
    *reg = *reg >> 1;
    flags.set_z_flag(*reg==0);
    flags.set_n_flag(false);
    flags.set_h_flag(false);
}
fn bit(index: u8, reg: &mut u8, flags: &mut Flags) {
    flags.set_n_flag(false);
    flags.set_h_flag(true);
    let bit_index = 0b0000_0001 << index;
    let is_set = (*reg & bit_index) != 0;
    flags.set_z_flag(is_set);
}
fn res(index: u8, reg: &mut u8) {
    *reg &= !(0b0000_0001 << index);
}
fn set(index: u8, reg: &mut u8) {
    *reg |= 0b0000_0001 << index;
}

pub struct Cpu {
    rom: Vec<u8>,
    regs: Registers,
    memory: Rc<RefCell<Memory>>,
}

impl Cpu {
    pub fn new(memory: Rc<RefCell<Memory>>, rom: Vec<u8>) -> Self {
        Self {
            rom,
            regs: Registers::new(),
            memory,
        }
    }

    fn write_data(&mut self, address: u16, data: u8) {
        self.memory.borrow_mut().write_u8(address, data);
    }
    fn write_data_u16(&mut self, address: Option<u16>, data: u16) {
        let address = match address {
            Some(a16) => a16,
            None => self.get_word(),
        };
        self.memory.borrow_mut().write_u16(address, data);
    }
    /// allows data to be collected from the ROM faithfully to how 
    /// CPUs actually work. calling `self.regs.pc()` implicitly increments it
    fn get_next(&mut self) -> u8 {
        self.rom[self.regs.pc()]
    }
    fn get_word(&mut self) -> u16 {
        combine_u8s(self.get_next(), self.get_next())
    }

    pub fn pop(&mut self) -> u16 {
        let lower = self.memory.borrow().load(self.regs.sp);
        self.regs.sp += 1;
        let higher = self.memory.borrow().load(self.regs.sp);
        self.regs.sp += 1;
        combine_u8s(lower, higher)
    }
    pub fn push(&mut self, data: u16) {
        self.regs.sp -= 2;
        self.memory.borrow_mut().write_u16(self.regs.sp, data);
    }

    fn hl_mem(&self) -> u8 {
        self.memory.borrow().load(self.regs.hl())
    }
    fn set_hl_mem(&self, data: u8) {
        self.memory.borrow_mut().write_u8(self.regs.hl(), data)
    }

    /// all the cpu opcodes which can be generalised to use any data
    /// most if not all are mathematical instructions and store result in acc
    fn add(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_u8_add(self.regs.a, data));
        self.regs.a += data;
        self.regs.f.set_z_flag(self.regs.a == 0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_c_flag(self.regs.a<data)    
    }
    fn adc(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_u8_add(self.regs.a, data));
        self.regs.a += data + self.regs.f.get_c_flag() as u8;
        self.regs.f.set_z_flag(self.regs.a == 0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_c_flag(self.regs.a<data);
    }
    fn sub(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_u8_sub(self.regs.a, data));
        self.regs.f.set_c_flag(self.regs.a<data);
        self.regs.a -= data;
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(true); 
    }
    fn sbc(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_u8_sub(self.regs.a, data));
        let comparison = self.regs.a;
        self.regs.a -= data + self.regs.f.get_c_flag() as u8;
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(true);
        self.regs.f.set_c_flag(self.regs.a>comparison);
    }
    fn and(&mut self, data: u8) {
        self.regs.a &= data;
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_h_flag(true);
        self.regs.f.set_c_flag(false);
    }
    fn xor(&mut self, data: u8) {
        self.regs.a ^= data;
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_h_flag(false);
        self.regs.f.set_c_flag(false);
    }
    fn or(&mut self, data: u8) {
        self.regs.a |= data;
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_h_flag(false);
        self.regs.f.set_c_flag(false);
    }
    fn cp(&mut self, data: u8) {
        let temp = self.regs.a;
        self.sub(data);
        self.regs.a = temp;
    }
    /// logic flow opcodes
    /// handle the returns, jumps and calls in the assembly
    fn jr(&mut self, cc: bool) {
        if !cc {
            return;
        }
        let jump = self.get_next() as i8;
        self.regs.jump_pc(jump);
    }
    fn ret(&mut self, cc: bool) {
        if !cc {
            return;
        }
        let address = self.pop();
        self.regs.set_pc(address);
    }
    /// all locs store the address in the assembly and not in registers
    /// so we can ask for it in the function to make it simpler
    fn call(&mut self, cc: bool) {
        let word = self.get_word();
        if cc {
            let next = self.regs.pc(); // instruction that we will jump back to on a return
            self.push(next as u16);
            self.regs.set_pc(word);
        }
    }
    fn jp(&mut self, cc: bool, address: Option<u16>) {
        let address = match address {
            Some(a) => a,
            None => self.get_word(),
        };
        if !cc {
            return;
        }
        print!("next address: {}", address);
        self.regs.set_pc(address);
    }

    pub fn process_next(&mut self) {  
        let opcode = self.get_next();
        print!("current opcode: {:x}, ", opcode);

        if opcode == 0xCB {
            self.process_prefixed();
            return;
        }
        match opcode {
            0x00 => {},
            0x01 => {let d=self.get_word(); self.regs.set_bc(d)}
            0x02 => self.write_data(self.regs.bc(), self.regs.a),
            0x03 => self.regs.set_bc(self.regs.bc() + 1),
            0x04 => inc(&mut self.regs.b, &mut self.regs.f),
            0x05 => dec(&mut self.regs.b, &mut self.regs.f),
            0x06 => self.regs.b = self.get_next(),
            0x07 => rlc(&mut self.regs.a, &mut self.regs.f),
            0x08 => self.write_data_u16(None, self.regs.sp),
            0x09 => {
                let r=add_u16(self.regs.hl(), self.regs.bc(), &mut self.regs.f); 
                self.regs.set_hl(r)
            },
            0x0A => self.regs.a = self.memory.borrow().load(self.regs.bc()),
            0x0B => self.regs.set_bc(self.regs.bc() - 1),
            0x0C => inc(&mut self.regs.c, &mut self.regs.f),
            0x0D => dec(&mut self.regs.c, &mut self.regs.f),
            0x0E => self.regs.c = self.get_next(),
            0x0F => rrc(&mut self.regs.a, &mut self.regs.f),
            0x10 => todo!(),
            0x11 => {
                let operand = self.get_word();
                self.regs.set_de(operand);
            }
            0x12 => self.write_data(self.regs.de(), self.regs.a),
            0x13 => self.regs.set_de(self.regs.de() + 1),
            0x14 => inc(&mut self.regs.d, &mut self.regs.f),
            0x15 => dec(&mut self.regs.d, &mut self.regs.f),
            0x16 => self.regs.d = self.get_next(),
            0x17 => rl(&mut self.regs.a, &mut self.regs.f),
            0x18 => self.jr(true), // this fn is conditional but still
            0x19 => {
                let r= add_u16(self.regs.hl(), self.regs.de(), &mut self.regs.f); 
                self.regs.set_hl(r)
            },
            0x1A => self.regs.a = self.memory.borrow().load(self.regs.de()),
            0x1B => self.regs.set_de(self.regs.de() - 1),
            0x1C => inc(&mut self.regs.e, &mut self.regs.f),
            0x1D => dec(&mut self.regs.e, &mut self.regs.f),
            0x1E => self.regs.e = self.get_next(),
            0x1F => rr(&mut self.regs.a, &mut self.regs.f),
            0x20 => self.jr(!self.regs.f.get_z_flag()),
            0x21 => {
                let operand = combine_u8s(self.get_next(), self.get_next());
                self.regs.set_hl(operand);
            }
            0x22 => {
                let address = self.regs.hl();
                self.memory.borrow_mut().write_u8(address, self.regs.a);
                self.regs.set_hl(self.regs.hl()+1);
            }
            0x23 => self.regs.set_hl(self.regs.hl()+1),
            0x24 => inc(&mut self.regs.h, &mut self.regs.f),
            0x25 => dec(&mut self.regs.h, &mut self.regs.f),
            0x26 => self.regs.h = self.get_next(),
            0x27 => todo!(),
            0x28 => self.jr(self.regs.f.get_z_flag()),
            0x29 => {
                let r=add_u16(self.regs.hl(), self.regs.hl(), &mut self.regs.f); 
                self.regs.set_hl(r)
            },
            0x2A => {
                self.regs.a = self.memory.borrow().load(self.regs.hl());
                self.regs.set_hl(self.regs.hl()+1);
            }
            0x2B => self.regs.set_hl(self.regs.hl()-1),
            0x2C => inc(&mut self.regs.l, &mut self.regs.f),
            0x2D => dec(&mut self.regs.l, &mut self.regs.f),
            0x2E => self.regs.l = self.get_next(),
            0x2F => self.regs.a = !self.regs.a,
            0x30 => self.jr(!self.regs.f.get_c_flag()),
            0x31 => self.regs.sp = combine_u8s(self.get_next(), self.get_next()),
            0x32 => {
                let address = self.regs.hl();
                self.memory.borrow_mut().write_u8(address, self.regs.a);
                self.regs.set_hl(self.regs.hl()+1);
            }
            0x33 => self.regs.sp += 1,
            0x34 => {
                let address = self.regs.hl();
                let mut data = self.memory.borrow().load(address);
                inc(&mut data, &mut self.regs.f);
                self.memory.borrow_mut().write_u8(address, data);
            }
            0x35 => {
                let address = self.regs.hl();
                let mut data = self.memory.borrow().load(address);
                dec(&mut data, &mut self.regs.f);
                self.memory.borrow_mut().write_u8(address, data);
            }
            0x36 => {
                let address = self.regs.hl();
                let data = self.get_next();
                self.memory.borrow_mut().write_u8(address, data);
            }
            0x37 => todo!(),
            0x38 => self.jr(self.regs.f.get_c_flag()),
            0x39 => {
                let r=add_u16(self.regs.hl(), self.regs.sp, &mut self.regs.f); 
                self.regs.set_hl(r)
            },
            0x3A => {
                self.regs.a = self.memory.borrow().load(self.regs.hl());
                self.regs.set_hl(self.regs.hl()-1);
            }
            0x3B => self.regs.sp -= 1,
            0x3C => inc(&mut self.regs.a, &mut self.regs.f),
            0x3D => dec(&mut self.regs.a, &mut self.regs.f),
            0x3E => self.regs.a = self.get_next(),
            0x3F => self.regs.f.set_c_flag(!self.regs.f.get_c_flag()),
            0x40 => {}, // redundant opcode "LD B, B"
            0x41 => self.regs.b = self.regs.c,
            0x42 => self.regs.b = self.regs.d,
            0x43 => self.regs.b = self.regs.e,
            0x44 => self.regs.b = self.regs.h,
            0x45 => self.regs.b = self.regs.l,
            0x46 => self.regs.b = self.hl_mem(),
            0x47 => self.regs.b = self.regs.a,
            0x48 => self.regs.c = self.regs.b,
            0x49 => {}, // redundant opcode "LD C, C"
            0x4A => self.regs.c = self.regs.d,
            0x4B => self.regs.c = self.regs.e,
            0x4C => self.regs.c = self.regs.h,
            0x4D => self.regs.c = self.regs.l,
            0x4E => self.regs.c = self.hl_mem(),
            0x4F => self.regs.c = self.regs.a,
            0x50 => self.regs.d = self.regs.b,
            0x51 => self.regs.d = self.regs.c,
            0x52 => {}, // redundant opcode "LD D, D"
            0x53 => self.regs.d = self.regs.e,
            0x54 => self.regs.d = self.regs.h,
            0x55 => self.regs.d = self.regs.l,
            0x56 => self.regs.d = self.hl_mem(),
            0x57 => self.regs.d = self.regs.a,
            0x58 => self.regs.e = self.regs.b,
            0x59 => self.regs.e = self.regs.c,
            0x5A => self.regs.e = self.regs.d,
            0x5B => {}, // redundant opcode "LD E, E"
            0x5C => self.regs.e = self.regs.h,
            0x5D => self.regs.e = self.regs.l,
            0x5E => self.regs.e = self.hl_mem(),
            0x5F => self.regs.e = self.regs.a,
            0x60 => self.regs.h = self.regs.b,
            0x61 => self.regs.h = self.regs.c,
            0x62 => self.regs.h = self.regs.d,
            0x63 => self.regs.h = self.regs.e,
            0x64 => {}, // redundant opcode "LD H, H"
            0x65 => self.regs.h = self.regs.l,
            0x66 => self.regs.h = self.hl_mem(),
            0x67 => self.regs.h = self.regs.a,
            0x68 => self.regs.l = self.regs.b,
            0x69 => self.regs.l = self.regs.c,
            0x6A => self.regs.l = self.regs.d,
            0x6B => self.regs.l = self.regs.e,
            0x6C => self.regs.l = self.regs.h,
            0x6D => {}, // redundant opcode "LD L, L"
            0x6E => self.regs.l = self.hl_mem(),
            0x6F => self.regs.l = self.regs.a,
            0x70 => self.set_hl_mem(self.regs.b),
            0x71 => self.set_hl_mem(self.regs.c),
            0x72 => self.set_hl_mem(self.regs.d),
            0x73 => self.set_hl_mem(self.regs.e),
            0x74 => self.set_hl_mem(self.regs.h),
            0x75 => self.set_hl_mem(self.regs.l),
            0x76 => todo!(),
            0x77 => self.set_hl_mem(self.regs.a),
            0x78 => self.regs.a = self.regs.b,
            0x79 => self.regs.a = self.regs.c,
            0x7A => self.regs.a = self.regs.d,
            0x7B => self.regs.a = self.regs.e,
            0x7C => self.regs.a = self.regs.h,
            0x7D => self.regs.a = self.regs.l,
            0x7E => self.regs.a = self.hl_mem(),
            0x7F => {}, // redundant opcode "LD A, A"
            0x80 => self.add(self.regs.b),
            0x81 => self.add(self.regs.c),
            0x82 => self.add(self.regs.d),
            0x83 => self.add(self.regs.e),
            0x84 => self.add(self.regs.h),
            0x85 => self.add(self.regs.l),
            0x86 => self.add(self.hl_mem()),
            0x87 => self.add(self.regs.a),
            0x88 => self.adc(self.regs.b),
            0x89 => self.adc(self.regs.c),
            0x8A => self.adc(self.regs.d),
            0x8B => self.adc(self.regs.e),
            0x8C => self.adc(self.regs.h),
            0x8D => self.adc(self.regs.l),
            0x8E => self.adc(self.hl_mem()),
            0x8F => self.adc(self.regs.a),
            0x90 => self.sub(self.regs.b),
            0x91 => self.sub(self.regs.c),
            0x92 => self.sub(self.regs.d),
            0x93 => self.sub(self.regs.e),
            0x94 => self.sub(self.regs.h),
            0x95 => self.sub(self.regs.l),
            0x96 => self.sub(self.hl_mem()),
            0x97 => self.sub(self.regs.a),
            0x98 => self.sbc(self.regs.b),
            0x99 => self.sbc(self.regs.c),
            0x9A => self.sbc(self.regs.d),
            0x9B => self.sbc(self.regs.e),
            0x9C => self.sbc(self.regs.h),
            0x9D => self.sbc(self.regs.l),
            0x9E => self.sbc(self.hl_mem()),
            0x9F => self.sbc(self.regs.a),
            0xA0 => self.and(self.regs.b),
            0xA1 => self.and(self.regs.c),
            0xA2 => self.and(self.regs.d),
            0xA3 => self.and(self.regs.e),
            0xA4 => self.and(self.regs.h),
            0xA5 => self.and(self.regs.l),
            0xA6 => self.and(self.hl_mem()),
            0xA7 => self.and(self.regs.a),
            0xA8 => self.xor(self.regs.b),
            0xA9 => self.xor(self.regs.c),
            0xAA => self.xor(self.regs.d),
            0xAB => self.xor(self.regs.e),
            0xAC => self.xor(self.regs.h),
            0xAD => self.xor(self.regs.l),
            0xAE => self.xor(self.hl_mem()),
            0xAF => self.xor(self.regs.a),
            0xB0 => self.or(self.regs.b),
            0xB1 => self.or(self.regs.c),
            0xB2 => self.or(self.regs.d),
            0xB3 => self.or(self.regs.e),
            0xB4 => self.or(self.regs.h),
            0xB5 => self.or(self.regs.l),
            0xB6 => self.or(self.hl_mem()),
            0xB7 => self.or(self.regs.a),
            0xB8 => self.cp(self.regs.b),
            0xB9 => self.cp(self.regs.c),
            0xBA => self.cp(self.regs.d),
            0xBB => self.cp(self.regs.e),
            0xBC => self.cp(self.regs.h),
            0xBD => self.cp(self.regs.l),
            0xBE => self.cp(self.hl_mem()),
            0xBF => self.cp(self.regs.a),
            0xC0 => self.ret(!self.regs.f.get_z_flag()),
            0xC1 => {
                let data = self.pop();
                self.regs.set_bc(data)
            },
            0xC2 => self.jp(!self.regs.f.get_z_flag(), None),
            0xC3 => self.jp(true, None),
            0xC4 => self.call(!self.regs.f.get_z_flag()),
            0xC5 => self.push(self.regs.bc()),
            0xC6 => {
                let val = self.get_next();
                self.add(val);
            }
            0xC7 => todo!(),
            0xC8 => self.ret(self.regs.f.get_z_flag()),
            0xC9 => self.ret(true),
            0xCA => self.jp(self.regs.f.get_z_flag(), None),
            0xCC => self.call(self.regs.f.get_z_flag()),
            0xCD => self.call(true),
            0xCE => {
                let data = self.get_next();
                self.add(data);
            }
            0xCF => todo!(),
            0xD0 => self.ret(!self.regs.f.get_c_flag()),
            0xD1 => {
                let data = self.pop();
                self.regs.set_bc(data);
            }
            0xD2 => self.jp(!self.regs.f.get_c_flag(), None),
            0xD4 => self.call(!self.regs.f.get_c_flag()),
            0xD5 => self.push(self.regs.de()),
            0xD6 => {
                let data = self.get_next();
                self.sub(data);
            }
            0xD7 => todo!(),
            0xD8 => self.ret(self.regs.f.get_c_flag()),
            0xD9 => todo!(),
            0xDA => self.jp(self.regs.f.get_c_flag(), None),
            0xDC => self.call(self.regs.f.get_c_flag()),
            0xDE => {
                let data = self.get_next();
                self.sbc(data);
            }
            0xDF => todo!(),
            0xE0 => {
                let address: u16 = combine_u8s(self.get_next(), 0xFF);
                self.memory.borrow_mut().write_u8(address, self.regs.a);
            }
            0xE1 => {
                let data = self.pop();
                self.regs.set_hl(data);
            }
            0xE2 => {
                let address: u16 = combine_u8s(self.regs.c, 0xFF);
                self.memory.borrow_mut().write_u8(address, self.regs.a);
            }
            0xE5 => self.push(self.regs.hl()),
            0xE6 => {
                let data = self.get_next();
                self.and(data);
            }
            0xE7 => todo!(),
            0xE8 => todo!(),
            0xE9 => self.jp(true, Some(self.regs.hl())),
            0xEA => {
                let word = self.get_word();
                self.memory.borrow_mut().write_u8(word, self.regs.a);
            }
            0xEE => {
                let data = self.get_next();
                self.xor(data);
            }
            0xEF => todo!(),
            0xF0 => {
                let address = combine_u8s(self.regs.a, 0xFF);
                self.regs.a = self.memory.borrow().load(address);
            }
            0xF1 => {
                let data = self.pop();
                self.regs.set_af(data)
            }
            0xF2 => {
                let address: u16 = combine_u8s(self.regs.a, 0xFF);
                self.regs.c = self.memory.borrow().load(address);
            }
            0xF3 => todo!(),
            0xF5 => self.push(self.regs.af()),
            0xF6 => {
                let data = self.get_next();
                self.or(data);
            }
            0xF7 => todo!(),
            0xF8 => todo!(),
            0xF9 => self.regs.sp = self.regs.hl(),
            0xFA => {
                let address = self.get_word();
                self.regs.a = self.memory.borrow().load(address);
            }
            0xFB => todo!(),
            0xFE => {
                let data = self.get_next();
                self.cp(data);
            }
            0xFF => todo!(),
            _ => panic!("unsupported opcode provided! you fucker"),
        }
    }

    fn process_prefixed(&mut self) {
        let opcode = self.get_next();
        let mut binding = self.hl_mem();
        let reg = match opcode % 8 {
            0 => &mut self.regs.b,
            1 => &mut self.regs.c,
            2 => &mut self.regs.d,
            3 => &mut self.regs.e,
            4 => &mut self.regs.h,
            5 => &mut self.regs.l,
            6 => &mut binding,
            7 => &mut self.regs.a,
            _ => panic!("broke maths")
        };
        let instruction = opcode / 8;
        match instruction {
            0 => rlc(reg, &mut self.regs.f),
            1 => rrc(reg, &mut self.regs.f),
            2 => rl(reg, &mut self.regs.f),
            3 => rr(reg, &mut self.regs.f),
            4 => sla(reg, &mut self.regs.f),
            5 => sra(reg, &mut self.regs.f),
            6 => swap(reg, &mut self.regs.f),
            7 => srl(reg, &mut self.regs.f),
            8..=15 => bit(instruction-8, reg, &mut self.regs.f),
            16..=23 => res(instruction-16, reg),
            24..=31 => set(instruction-24, reg),
            _ => panic!("invalid instruction")
        }
    }
}