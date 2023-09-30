use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;
use crate::{registers::*, combine_u8s};

const Z_FLAG: u8 = 7;
const N_FLAG: u8 = 6;
//https://robdor.com/2016/08/10/gameboy-emulator-half-carry-flag/ goddamn is that smart
const H_FLAG: u8 = 5;
const C_FLAG: u8 = 4; 

fn half_carry_u8(a: u8, b: u8) -> bool {
    (((a & 0xf) + (b & 0xf)) & 0x10) == 0x10
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

    fn hl_mem(&self) -> u8 {
        self.memory.borrow().load(self.regs.hl())
    }
    fn set_hl_mem(&self, data: u8) {
        self.memory.borrow_mut().write_u8(self.regs.hl(), data)
    }

    // loading opcodes and words
    fn get_next(&mut self) -> u8 {
        self.rom[self.regs.pc()]
    }
    fn get_word(&mut self) -> u16 {
        combine_u8s(self.get_next(), self.get_next())
    }

    /// the actual reusable opcodes
    /// they don't contribute to the (m/t)-cycles
    fn inc(&mut self, data: u8) -> u8 {
        self.regs.set_z_flag(data+1 == 0);
        self.regs.set_n_flag(false);
        let half_carried = half_carry_u8(data, 1); 
        self.regs.set_h_flag(half_carried);
        data+1
    }
    fn dec(&mut self, data: u8) -> u8 {
        let ndata = data - 1;
        self.regs.set_z_flag(ndata == 0);
        self.regs.set_n_flag(true);
        self.regs.set_h_flag(ndata.trailing_zeros() > 3);
        ndata
    }
    fn rr(&mut self, reg: &mut u8) {
        let temp = *reg & 0b0000_0001;
        *reg = reg.rotate_right(1) ^ ((self.regs.get_c_flag() as u8)<<7);
        self.regs.set_c_flag(temp>0);
        self.regs.set_n_flag(false);
        self.regs.set_h_flag(false);
        self.regs.set_z_flag(*reg==0);
    }
    fn rl(&mut self, reg: &mut u8) {
        let temp = *reg & 0b1000_0000;
        *reg = reg.rotate_left(1) ^ (self.regs.get_c_flag() as u8);
        self.regs.set_c_flag(temp>0);
        self.regs.set_n_flag(false);
        self.regs.set_h_flag(false);
    }
    fn rlc(&mut self, reg: &mut u8) {
        *reg = reg.rotate_left(1);
        self.regs.set_c_flag(*reg&0b0000_0001>0); // checks if 1st bit is set
        self.regs.set_h_flag(false);
        self.regs.set_n_flag(false);
        self.regs.set_z_flag(*reg==0);
    }
    fn rrca(&mut self, reg: &mut u8) {
        *reg = reg.rotate_right(1);
        self.regs.set_c_flag(*reg&0b1000_0000>0); // checks if 7th bit is set
        self.regs.set_h_flag(false);
        self.regs.set_n_flag(false);
        self.regs.set_z_flag(*reg==0);
    }
    fn add(&mut self, data: u8) {
        self.regs.set_h_flag(half_carry_u8(self.regs.a, data));
        self.regs.a += data;
        self.regs.set_z_flag(self.regs.a == 0);
        self.regs.set_n_flag(false);
        self.regs.set_c_flag(self.regs.a<data)
    }
    fn adc(&mut self, data: u8) {
        self.regs.set_h_flag(half_carry_u8(self.regs.a, data));
        self.regs.a += data + self.regs.get_c_flag() as u8;
        self.regs.set_z_flag(self.regs.a == 0);
        self.regs.set_n_flag(false);
        self.regs.set_c_flag(self.regs.a<data);
    }
    fn sub(&mut self, data: u8) {
        self.regs.set_h_flag(half_carry_u8(self.regs.a, data));
        self.regs.set_c_flag(self.regs.a<data);
        self.regs.a -= data;
        self.regs.set_z_flag(self.regs.a==0);
        self.regs.set_n_flag(true); 
    }
    fn sbc(&mut self, data: u8) {
        let comparison = self.regs.a;
        self.regs.set_h_flag(half_carry_u8(self.regs.a, data));
        self.regs.a -= data + self.regs.get_c_flag() as u8;
        self.regs.set_z_flag(self.regs.a==0);
        self.regs.set_n_flag(true);
        self.regs.set_c_flag(self.regs.a>comparison);
    }
    fn and(&mut self, data: u8) {
        self.regs.a &= data;
        self.regs.set_z_flag(self.regs.a==0);
        self.regs.set_n_flag(false);
        self.regs.set_h_flag(true);
        self.regs.set_c_flag(false);
    }
    fn xor(&mut self, data: u8) {
        self.regs.a ^= data;
        self.regs.set_z_flag(self.regs.a==0);
        self.regs.set_n_flag(false);
        self.regs.set_h_flag(false);
        self.regs.set_c_flag(false);
    }
    fn or(&mut self, data: u8) {
        self.regs.a |= data;
        self.regs.set_z_flag(self.regs.a==0);
        self.regs.set_n_flag(false);
        self.regs.set_h_flag(false);
        self.regs.set_c_flag(false);
    }
    /// it is used for flags but dont want the a to update
    fn cp(&mut self, data: u8) {
        let temp = self.regs.a;
        self.sub(data);
        self.regs.a = temp;
    }

    pub fn process_next(&mut self) {  
        let opcode = self.get_next();
        if opcode == 0xCB {
            self.process_prefixed();
            return;
        }
        match opcode {
            0x00 => {},
            0x01 => {let d=self.get_word(); self.regs.set_bc(d)}
            0x02 => {
                let address = self.regs.bc();
                self.memory.borrow_mut().write_u8(address, self.regs.a)
            },
            0x03 => self.regs.set_bc(self.regs.bc() + 1),
            0x04 => self.regs.b = self.inc(self.regs.b),
            0x05 => self.regs.b = self.dec(self.regs.b),
            0x06 => self.regs.b = self.get_next(),
            0x07 => {
                let reg = &mut self.regs.a;
                self.rlc(reg)
            },
            0x08 => {
                let operand = combine_u8s(self.get_next(), self.get_next());
                self.memory.borrow_mut().write_u16(operand, self.regs.sp)
            }
            0x09 => todo!(),
            0x0A => self.regs.a = self.memory.borrow().load(self.regs.bc()),
            0x0B => self.regs.set_bc(self.regs.bc() - 1),
            0x0C => self.regs.c = self.inc(self.regs.c),
            0x0D => self.regs.c = self.dec(self.regs.c),
            0x0E => self.regs.c = self.get_next(),
            0x0F => todo!(), //self.rrca(),
            0x10 => todo!(),
            0x11 => {
                let operand = combine_u8s(self.get_next(), self.get_next());
                self.regs.set_de(operand);
            }
            0x12 => {
                let address = self.regs.de();
                self.memory.borrow_mut().write_u8(address, self.regs.a);
            }
            0x13 => self.regs.set_de(self.regs.de() + 1),
            0x14 => self.regs.d = self.inc(self.regs.d),
            0x15 => self.regs.d = self.dec(self.regs.d),
            0x16 => self.regs.d = self.get_next(),
            0x17 => todo!(), //self.rla(),
            0x18 => todo!(),
            0x19 => todo!(),
            0x1A => self.regs.a = self.memory.borrow().load(self.regs.de()),
            0x1B => self.regs.set_de(self.regs.de() - 1),
            0x1C => self.regs.e = self.inc(self.regs.e),
            0x1D => self.regs.e = self.dec(self.regs.e),
            0x1E => self.regs.e = self.get_next(),
            0x1F => todo!(), //self.rra(),
            0x20 => todo!(),
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
            0x24 => self.regs.h = self.inc(self.regs.h),
            0x25 => self.regs.h = self.dec(self.regs.h),
            0x26 => self.regs.h = self.get_next(),
            0x27 => todo!(),
            0x28 => todo!(),
            0x29 => todo!(),
            0x2A => {
                self.regs.a = self.memory.borrow().load(self.regs.hl());
                self.regs.set_hl(self.regs.hl()+1);
            }
            0x2B => self.regs.set_hl(self.regs.hl()-1),
            0x2C => self.regs.l = self.inc(self.regs.l),
            0x2D => self.regs.l = self.dec(self.regs.l),
            0x2E => self.regs.l = self.get_next(),
            0x2F => todo!(),
            0x30 => todo!(),
            0x31 => self.regs.sp = combine_u8s(self.get_next(), self.get_next()),
            0x32 => {
                let address = self.regs.hl();
                self.memory.borrow_mut().write_u8(address, self.regs.a);
                self.regs.set_hl(self.regs.hl()+1);
            }
            0x33 => self.regs.sp += 1,
            0x34 => {
                let address = self.regs.hl();
                let data = self.memory.borrow().load(address);
                let inc = self.inc(data);
                self.memory.borrow_mut().write_u8(address, inc);
            }
            0x35 => {
                let address = self.regs.hl();
                let data = self.memory.borrow().load(address);
                let dec = self.dec(data);
                self.memory.borrow_mut().write_u8(address, dec);
            }
            0x36 => {
                let address = self.regs.hl();
                let data = self.get_next();
                self.memory.borrow_mut().write_u8(address, data);
            }
            0x37 => todo!(),
            0x38 => todo!(),
            0x39 => todo!(),
            0x3A => {
                self.regs.a = self.memory.borrow().load(self.regs.hl());
                self.regs.set_hl(self.regs.hl()-1);
            }
            0x3B => self.regs.sp -= 1,
            0x3C => self.regs.a = self.inc(self.regs.a),
            0x3D => self.regs.a = self.dec(self.regs.a),
            0x3E => self.regs.a = self.get_next(),
            0x3F => todo!(),
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
            _ => panic!("unsupported opcode provided!"),
        }
    }

    fn process_prefixed(&mut self) {
        let opcode = self.get_next();
        let _reg = match opcode % 8 {
            0 => &mut self.regs.b,
            1 => &mut self.regs.c,
            2 => &mut self.regs.d,
            3 => &mut self.regs.e,
            4 => &mut self.regs.h,
            5 => &mut self.regs.l,
            //6 => binding.load_mut(self.regs.hl()),
            7 => &mut self.regs.a,
            _ => panic!("broke maths")
        };
        //let instruction = match opcode / 8 {
        //    1 => 
        //}
    }
}