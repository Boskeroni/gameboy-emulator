use core::panic;
use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;
use crate::{registers::*, combine_u8s};
use crate::opcodes::*;

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
    fn read_mem(&self, address: u16) -> u8 {
        self.memory.borrow().read(address)
    }
    fn write_mem(&mut self, address: u16, data: u8) {
        self.memory.borrow_mut().write_u8(address, data);
    }
    fn write_mem_u16(&mut self, address: Option<u16>, data: u16) {
        let address = match address {
            Some(a16) => a16,
            None => self.get_word(),
        };
        self.memory.borrow_mut().write_u16(address, data);
    }
    fn hl_mem(&self) -> u8 {
        self.read_mem(self.regs.hl())
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
        let lower = self.read_mem(self.regs.sp);
        self.regs.sp += 1;
        let higher = self.read_mem(self.regs.sp);
        self.regs.sp += 1;
        combine_u8s(lower, higher)
    }
    pub fn push(&mut self, data: u16) {
        self.regs.sp -= 2;
        self.memory.borrow_mut().write_u16(self.regs.sp, data);
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
        // still have to load it so we update pc even if no jump
        let jump = self.get_next() as i8;
        if !cc {
            return;
        }
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
        print!("current opcode: {opcode:x}, ");
        print!("pc: {}", self.regs.pc);

        if opcode == 0xCB {
            self.process_prefixed();
            return;
        }
        match opcode {
            0x00 => {},
            0x01 => {let d=self.get_word(); self.regs.set_bc(d)}
            0x02 => self.write_mem(self.regs.bc(), self.regs.a),
            0x03 => self.regs.set_bc(self.regs.bc() + 1),
            0x04 => inc(&mut self.regs.b, &mut self.regs.f),
            0x05 => dec(&mut self.regs.b, &mut self.regs.f),
            0x06 => self.regs.b = self.get_next(),
            0x07 => rlc(&mut self.regs.a, &mut self.regs.f),
            0x08 => self.write_mem_u16(None, self.regs.sp),
            0x09 => {
                let r=add_u16(self.regs.hl(), self.regs.bc(), &mut self.regs.f); 
                self.regs.set_hl(r)
            },
            0x0A => self.regs.a = self.read_mem(self.regs.bc()),
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
            0x12 => self.write_mem(self.regs.de(), self.regs.a),
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
            0x1A => self.regs.a = self.read_mem(self.regs.de()),
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
                self.regs.a = self.read_mem(self.regs.hl());
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
                let mut data = self.read_mem(address);
                inc(&mut data, &mut self.regs.f);
                self.memory.borrow_mut().write_u8(address, data);
            }
            0x35 => {
                let address = self.regs.hl();
                let mut data = self.read_mem(address);
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
                self.regs.a = self.read_mem(self.regs.hl());
                self.regs.set_hl(self.regs.hl()-1);
            }
            0x3B => self.regs.sp -= 1,
            0x3C => inc(&mut self.regs.a, &mut self.regs.f),
            0x3D => dec(&mut self.regs.a, &mut self.regs.f),
            0x3E => self.regs.a = self.get_next(),
            0x3F => self.regs.f.set_c_flag(!self.regs.f.get_c_flag()),
            0x76 => todo!(),
            0x40..=0x7F => {
                
            }
            0x80..=0xBF => {
                // all the maths instructions, alot of repeat
                let param = match opcode % 8 {
                    0 => self.regs.b,
                    1 => self.regs.c,
                    2 => self.regs.d,
                    3 => self.regs.e,
                    4 => self.regs.h,
                    5 => self.regs.l,
                    6 => self.hl_mem(),
                    7 => self.regs.a,
                    _ => panic!("maths is fucky")
                };
                match opcode / 8 {
                    0 => self.add(param),
                    1 => self.adc(param),
                    2 => self.sub(param),
                    3 => self.sbc(param),
                    4 => self.and(param),
                    5 => self.xor(param),
                    6 => self.or(param),
                    7 => self.cp(param),
                    _ => panic!("fucky wucky maths")
                }
            }
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
                self.regs.a = self.read_mem(address);
            }
            0xF1 => {
                let data = self.pop();
                self.regs.set_af(data)
            }
            0xF2 => {
                let address: u16 = combine_u8s(self.regs.a, 0xFF);
                self.regs.c = self.read_mem(address);
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
                self.regs.a = self.read_mem(address);
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