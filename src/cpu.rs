use core::panic;
use std::fs::File;
use std::io::Write;
use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;
use crate::{registers::*, combine_u8s};
use crate::opcodes::*;

pub struct Cpu {
    regs: Registers,
    memory: Rc<RefCell<Memory>>,
    stopped: bool,
    ime: bool,
}

impl Cpu {
    pub fn new(memory: Rc<RefCell<Memory>>) -> Self {
        Self {
            regs: Registers::new(),
            memory,
            stopped: false,
            ime: false,
        }
    }
    fn read_mem(&self, address: u16) -> u8 {
        self.memory.borrow().read(address)
    }
    fn read_u16(&self, address: u16) -> u16 {
        combine_u8s(self.read_mem(address), self.read_mem(address+1))
    }
    fn write_mem(&self, address: u16, data: u8) {
        self.memory.borrow_mut().write(address, data);
    }
    fn write_mem_u16(&mut self, address: Option<u16>, data: u16) {
        let address = match address {
            Some(a16) => a16,
            None => self.get_word(),
        };
        self.memory.borrow_mut().write_u16(address, data);
    }

    /// allows data to be collected from the ROM faithfully to how 
    /// CPUs actually work. calling `self.regs.pc()` implicitly increments it
    fn get_next(&mut self) -> u8 {
        self.memory.borrow().read(self.regs.pc())
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
        self.write_mem_u16(Some(self.regs.sp), data);
    }

    /// all the cpu opcodes which can be generalised to use any data
    /// most if not all are mathematical instructions and store result in acc
    fn add(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_add(self.regs.a, data));
        self.regs.a = self.regs.a.wrapping_add(data);
        self.regs.f.set_z_flag(self.regs.a == 0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_c_flag(self.regs.a<data)    
    }
    fn adc(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_add(self.regs.a, data));
        self.regs.a = self.regs.a.wrapping_add(data + self.regs.f.c_flag() as u8);
        self.regs.f.set_z_flag(self.regs.a == 0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_c_flag(self.regs.a<data);
    }
    fn sub(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_sub(self.regs.a, data));
        self.regs.f.set_c_flag(self.regs.a<data);
        self.regs.a = self.regs.a.wrapping_sub(data);
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(true); 
    }
    fn sbc(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_sub(self.regs.a, data));
        let comparison = self.regs.a;
        self.regs.a = self.regs.a.wrapping_sub(data + self.regs.f.c_flag() as u8);
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
    fn add_u16(&mut self, r1: u16, r2: u16) -> u16 {
        let sum = r1.wrapping_add(r2);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_h_flag(half_carry_u16(r1, r2));
        self.regs.f.set_c_flag(sum<r1);
        sum
    }
    fn add_sp(&mut self, r2: i8) -> u16 {
        let res = if r2 >= 0 {
            self.regs.sp + (r2 as u16)
        } else {
            self.regs.sp - (r2.abs() as u16)
        };
        self.regs.f.set_n_flag(false);
        self.regs.f.set_z_flag(false);

        res
    }
    fn daa(&mut self) {
        if !self.regs.f.n_flag() {
            // need to change the lower nybble
            if self.regs.f.h_flag() || self.regs.a&0xF > 9 {
                self.regs.a += 6;
            }
            // need to change the upper nybble
            // we add 0x60 since the carry will reset it
            if self.regs.f.c_flag() || self.regs.a > 0x99 {
                self.regs.a = self.regs.a.wrapping_add(0x60);
                self.regs.f.set_c_flag(true);
            }
        } else {
            if self.regs.a &0xF0 > 0x90 {
                self.regs.a -= 0x60;
            }
            if self.regs.a &0xF > 0x9 {
                self.regs.a -= 6;
            }
        }
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
    fn call(&mut self, cc: bool, address: Option<u8>) {
        let address = match address {
            Some(v) => v as u16,
            None => self.get_word()
        };
        if !cc {
            return;
        }

        // updating the stack pointer accordingly
        let next = self.regs.pc(); // instruction that we will jump back to on a return
        self.push(next as u16);
        self.regs.set_pc(address);
    }
    fn jp(&mut self, cc: bool, address: Option<u16>) {
        let address = match address {
            Some(a) => a,
            None => self.get_word(),
        };
        if !cc {
            return;
        }
        if address == 0xC000 {
            let mut debug_file = File::create("debug.gb").unwrap();
            debug_file.write_all(&self.memory.borrow().memory).unwrap();
        }
        self.regs.set_pc(address);
    }

    pub fn process_next(&mut self) {          
        let opcode = self.get_next();
        if opcode == 0xCB {
            self.process_prefixed();
            return;
        }
        match opcode {
            0x00 => {}, // NOP
            0x01 => {let w=self.get_word(); self.regs.set_bc(w)} // LD BC, n16
            0x02 => self.write_mem(self.regs.bc(), self.regs.a), // LD (BC), A
            0x03 => self.regs.set_bc(self.regs.bc() + 1), // INC BC
            0x04 => inc(&mut self.regs.b, &mut self.regs.f), // INC B
            0x05 => dec(&mut self.regs.b, &mut self.regs.f), // DEC B
            0x06 => self.regs.b = self.get_next(), // LD B, n8
            0x07 => rlc(&mut self.regs.a, &mut self.regs.f), // RLCA
            0x08 => self.write_mem_u16(None, self.regs.sp), // LD (a16), SP
            0x09 => {let r=self.add_u16(self.regs.hl(), self.regs.bc()); self.regs.set_hl(r)}, // ADD HL, BC
            0x0A => self.regs.a = self.read_mem(self.regs.bc()), // LD A, (BC)
            0x0B => self.regs.set_bc(self.regs.bc() - 1), // DEC BC
            0x0C => inc(&mut self.regs.c, &mut self.regs.f), // INC C
            0x0D => dec(&mut self.regs.c, &mut self.regs.f), // DEC C
            0x0E => self.regs.c = self.get_next(), // LD C, n8
            0x0F => rrc(&mut self.regs.a, &mut self.regs.f), // RRCA
            0x10 => {self.stopped = true; self.get_next();}, // STOP 0
            0x11 => {let w = self.get_word(); self.regs.set_de(w)} // LD DE, n16
            0x12 => self.write_mem(self.regs.de(), self.regs.a), // LD (DE), A
            0x13 => self.regs.set_de(self.regs.de() + 1), // INC DE
            0x14 => inc(&mut self.regs.d, &mut self.regs.f), // INC D
            0x15 => dec(&mut self.regs.d, &mut self.regs.f), // DEC D
            0x16 => self.regs.d = self.get_next(), // LD D, n8
            0x17 => rl(&mut self.regs.a, &mut self.regs.f), // RLA
            0x18 => self.jr(true), // JR i8
            0x19 => {let r= self.add_u16(self.regs.hl(), self.regs.de()); self.regs.set_hl(r)}, // ADD HL, DE
            0x1A => self.regs.a = self.read_mem(self.regs.de()), // LD A, (DE)
            0x1B => self.regs.set_de(self.regs.de() - 1), // DEC DE
            0x1C => inc(&mut self.regs.e, &mut self.regs.f), // INC E
            0x1D => dec(&mut self.regs.e, &mut self.regs.f), // DEC E
            0x1E => self.regs.e = self.get_next(), // LD E n8
            0x1F => rr(&mut self.regs.a, &mut self.regs.f), // RRA
            0x20 => self.jr(!self.regs.f.z_flag()), // JR NZ, i8
            0x21 => {let w = self.get_word(); self.regs.set_hl(w);} //LD HL, n16
            0x22 => {let hl=self.regs.hli(); self.write_mem(hl, self.regs.a);} // LD (HL+), A
            0x23 => {self.regs.hli();}, // INC HL
            0x24 => inc(&mut self.regs.h, &mut self.regs.f), // INC H
            0x25 => dec(&mut self.regs.h, &mut self.regs.f), // DEC H
            0x26 => self.regs.h = self.get_next(), // LD H, n8
            0x27 => self.daa(), // DAA
            0x28 => self.jr(self.regs.f.z_flag()), // JR Z, i8
            0x29 => {let r=self.add_u16(self.regs.hl(), self.regs.hl()); self.regs.set_hl(r)}, // ADD HL, HL
            0x2A => {let hl = self.regs.hli(); self.regs.a = self.read_mem(hl)}, // LD A, (HL+)
            0x2B => self.regs.set_hl(self.regs.hl()-1), // DEC HL
            0x2C => inc(&mut self.regs.l, &mut self.regs.f), // INC L
            0x2D => dec(&mut self.regs.l, &mut self.regs.f), // DEC L
            0x2E => self.regs.l = self.get_next(), // LD L, n8
            0x2F => self.regs.a = !self.regs.a, // CPL
            0x30 => self.jr(!self.regs.f.c_flag()), // JR NC, i8
            0x31 => self.regs.sp = combine_u8s(self.get_next(), self.get_next()), // LD SP, n16
            0x32 => {let hl = self.regs.hli(); self.write_mem(hl, self.regs.a)} // LD (HL-), A
            0x33 => self.regs.sp += 1, // INC SP
            0x34 => {
                let mut data = self.read_mem(self.regs.hl());
                inc(&mut data, &mut self.regs.f);
                self.write_mem(self.regs.hl(), data);
            } // INC (HL)
            0x35 => {
                let mut data = self.read_mem(self.regs.hl());
                dec(&mut data, &mut self.regs.f);
                self.write_mem(self.regs.hl(), data);
            } // DEC (HL)
            0x36 => {let o = self.get_next(); self.write_mem(self.regs.hl(), o)} // LD (HL), n8
            0x37 => self.regs.f.set_c_flag(true), // SCF
            0x38 => self.jr(self.regs.f.c_flag()), // JR C, i8
            0x39 => {let r= self.add_u16(self.regs.hl(), self.regs.sp); self.regs.set_hl(r)}, // ADD HL, SP
            0x3A => {let hl = self.regs.hld(); self.regs.a = self.read_mem(hl)} // LD A, (HL-)
            0x3B => self.regs.sp -= 1, // DEC SP
            0x3C => inc(&mut self.regs.a, &mut self.regs.f), // INC A
            0x3D => dec(&mut self.regs.a, &mut self.regs.f), // DEC A
            0x3E => self.regs.a = self.get_next(), // LD A, n8
            0x3F => self.regs.f.set_c_flag(!self.regs.f.c_flag()), // CCF
            0x76 => self.stopped = true, // HALT
            0x40..=0x7F => {
                // the LD assignments are all just repeatable
                let src = match opcode % 8 {
                    0 => self.regs.b,
                    1 => self.regs.c,
                    2 => self.regs.d,
                    3 => self.regs.e,
                    4 => self.regs.h,
                    5 => self.regs.l,
                    6 => self.read_mem(self.regs.hl()),
                    7 => self.regs.a,
                    _ => panic!("broke maths"),
                };
                match (opcode-0x40) / 8 {
                    0 => self.regs.b = src,
                    1 => self.regs.c = src,
                    2 => self.regs.d = src,
                    3 => self.regs.e = src,
                    4 => self.regs.h = src,
                    5 => self.regs.l = src,
                    6 => self.write_mem(self.regs.hl(), src),
                    7 => self.regs.a = src,
                    _ => panic!("invalid opcode"),
                };
            } // LD *, *
            0x80..=0xBF => {
                // all the maths instructions, alot of repeat
                let param = match opcode % 8 {
                    0 => self.regs.b,
                    1 => self.regs.c,
                    2 => self.regs.d,
                    3 => self.regs.e,
                    4 => self.regs.h,
                    5 => self.regs.l,
                    6 => self.read_mem(self.regs.hl()),
                    7 => self.regs.a,
                    _ => panic!("maths is fucky")
                };
                match (opcode-0x80) / 8 {
                    0 => self.add(param),
                    1 => self.adc(param),
                    2 => self.sub(param),
                    3 => self.sbc(param),
                    4 => self.and(param),
                    5 => self.xor(param),
                    6 => self.or(param),
                    7 => self.cp(param),
                    _ => panic!("fucky maths")
                }
            } // * A, *
            0xC0 => self.ret(!self.regs.f.z_flag()), // RET NZ
            0xC1 => {let p = self.pop(); self.regs.set_bc(p)}, // POP BC
            0xC2 => self.jp(!self.regs.f.z_flag(), None), // JP NZ, n16
            0xC3 => self.jp(true, None), // JP n16
            0xC4 => self.call(!self.regs.f.z_flag(), None), // CALL NZ n16
            0xC5 => self.push(self.regs.bc()), // PUSH BC
            0xC6 => {let o = self.get_next(); self.add(o)} // ADD A, n8
            0xC7 => self.call(true, Some(0x00)), // RST 00
            0xC8 => self.ret(self.regs.f.z_flag()), // RET Z
            0xC9 => self.ret(true), // RET
            0xCA => self.jp(self.regs.f.z_flag(), None), // JP Z, n16
            0xCC => self.call(self.regs.f.z_flag(), None), // CALL Z, n16
            0xCD => self.call(true, None), // CALL n16
            0xCE => {let o = self.get_next(); self.adc(o)} // ADC A, n8
            0xCF => self.call(true, Some(0x08)), // RST 08
            0xD0 => self.ret(!self.regs.f.c_flag()), // RET NC
            0xD1 => {let p = self.pop(); self.regs.set_de(p)} // POP DE 
            0xD2 => self.jp(!self.regs.f.c_flag(), None), // JP NC, n16
            0xD4 => self.call(!self.regs.f.c_flag(), None), // CALL NC, n16
            0xD5 => self.push(self.regs.de()), // PUSH DE
            0xD6 => {let o = self.get_next(); self.sub(o)} // SUB n8
            0xD7 => self.call(true, Some(0x10)), // CALL 10
            0xD8 => self.ret(self.regs.f.c_flag()), // RET C
            0xD9 => {self.ret(true); self.ime = true;}, // RETI
            0xDA => self.jp(self.regs.f.c_flag(), None), // JP C, n16
            0xDC => self.call(self.regs.f.c_flag(), None), // CALL C, n16
            0xDE => {let o = self.get_next(); self.sbc(o)} // SBC A, n8
            0xDF => self.call(true, Some(0x18)), // RST 18
            0xE0 => {let a = combine_u8s(self.get_next(), 0xFF); self.write_mem(a, self.regs.a)} // LDH (a8), A
            0xE1 => {let o = self.pop(); self.regs.set_hl(o)} // POP HL
            0xE2 => {let a = combine_u8s(self.regs.c, 0xFF); self.write_mem(a, self.regs.a)} // LD (C), A
            0xE5 => self.push(self.regs.hl()), // PUSH HL
            0xE6 => {let o = self.get_next(); self.and(o)} // AND A, n8
            0xE7 => self.call(true, Some(0x20)), // RST 20
            0xE8 => {let o = self.get_next(); self.regs.sp = self.add_sp(o as i8)}, // ADD SP, n8
            0xE9 => self.jp(true, Some(self.read_u16(self.regs.hl()))), // JP (HL)
            0xEA => {let w = self.get_word(); self.write_mem(w, self.regs.a)} // LD (n16), A
            0xEE => {let o = self.get_next(); self.xor(o)} // XOR A, n8
            0xEF => self.call(true, Some(0x28)), // RST 28
            0xF0 => self.regs.a = self.read_mem(combine_u8s(self.regs.a, 0xFF)), // LDH A, (n8)
            0xF1 => {let p = self.pop(); self.regs.set_af(p)} // POP AF
            0xF2 => self.regs.a = self.read_mem(combine_u8s(self.regs.c, 0xFF)), // LD A, (C)
            0xF3 => self.ime = false, // DI
            0xF5 => self.push(self.regs.af()), // PUSH AF
            0xF6 => {let o = self.get_next(); self.or(o)} // OR A, n8
            0xF7 => self.call(true, Some(0x30)), // RST 30
            0xF8 => {let o = self.get_next(); let sp = self.add_sp(o as i8); self.regs.set_hl(sp)}, // LD HL, SP+n8
            0xF9 => self.regs.sp = self.regs.hl(), // LD SP, HL
            0xFA => {let w = self.get_word(); self.regs.a = self.read_mem(w)} // LD A, (a16)
            0xFB => self.ime = true, // EI
            0xFE => {let o = self.get_next(); self.cp(o)} // CP n8
            0xFF => self.call(true, Some(0x38)), // RST 38
            _ => panic!("unsupported opcode provided! you fucker"),
        }
    }

    fn process_prefixed(&mut self) {
        fn run_prefixed(dst: &mut u8, flags: &mut Flags, i: u8) {
            match i {
                0 => rlc(dst, flags),
                1 => rrc(dst, flags),
                2 => rl(dst, flags),
                3 => rr(dst, flags),
                4 => sla(dst, flags),
                5 => sra(dst, flags),
                6 => swap(dst, flags),
                7 => srl(dst, flags),
                8..=15 => bit(i-8, dst, flags),
                16..=23 => res(i-16, dst),
                24..=31 => set(i-24, dst),
                _ => panic!("invalid instruction")
            }
        }
        let opcode = self.get_next();
        let instruction = opcode / 8;
        let data_src = opcode % 8;

        // this works with memory so needs to be handled differently
        if data_src == 6 {
            let data = &mut self.read_mem(self.regs.hl());
            run_prefixed(data, &mut self.regs.f, instruction);
            self.write_mem(self.regs.hl(), *data);
            return;
        }

        let dst = match data_src {
            0 => &mut self.regs.b,
            1 => &mut self.regs.c,
            2 => &mut self.regs.d,
            3 => &mut self.regs.e,
            4 => &mut self.regs.h,
            5 => &mut self.regs.l,
            7 => &mut self.regs.a,
            _ => panic!("broke maths")
        };
        run_prefixed(dst, &mut self.regs.f, instruction);
    }
}