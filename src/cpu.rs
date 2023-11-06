use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;
use crate::{combine_u8s, split_u16};
use crate::opcodes::*;
use crate::registers::*;

pub struct Cpu {
    regs: Registers,
    memory: Rc<RefCell<Memory>>,
    stopped: bool,
    scheduled_ime: bool,
    ime: bool,
    used: Vec<u8>,
}

impl Cpu {
    pub fn new(memory: Rc<RefCell<Memory>>) -> Self {
        Self {
            regs: Registers::new(),
            memory,
            stopped: false,
            scheduled_ime: false,
            ime: false,
            used: Vec::new(),
        }
    }
    fn read(&self, address: u16) -> u8 {
        self.memory.borrow().read(address)
    }
    fn read_u16(&self, address: u16) -> u16 {
        combine_u8s(self.read(address), self.read(address+1))
    }
    fn write(&self, address: u16, data: u8) {
        self.memory.borrow_mut().write(address, data);
    }
    fn write_u16(&mut self, address: Option<u16>, data: u16) {
        let address = match address {
            Some(a16) => a16,
            None => self.next_word(),
        };
        let (upper, lower) = split_u16(data);
        self.write(address, lower);
        self.write(address+1, upper);
    }

    /// allows data to be collected from the ROM faithfully to how 
    /// CPUs actually work. 
    /// 
    /// calling `self.regs.pc()` implicitly increments it
    fn next_byte(&mut self) -> u8 {
        self.memory.borrow().read(self.regs.pc())
    }
    fn next_word(&mut self) -> u16 {
        combine_u8s(self.next_byte(), self.next_byte())
    }

    fn pop(&mut self) -> u16 {
        let lower = self.read(self.regs.sp);
        let higher = self.read(self.regs.sp+1);
        let answer = combine_u8s(lower, higher);
        self.regs.sp += 2;
        answer
    }
    fn push(&mut self, data: u16) {
        self.regs.sp -= 2;
        self.write_u16(Some(self.regs.sp), data);
    }

    /// all the cpu opcodes which can be generalised to use any data
    /// most if not all are mathematical instructions and store result in acc
    fn add(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_add(self.regs.a, data));
        let (result, carried) = self.regs.a.overflowing_add(data);
        self.regs.a = result;
        self.regs.f.set_z_flag(self.regs.a == 0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_c_flag(carried)    
    }
    fn adc(&mut self, data: u8) {
        let add = data + self.regs.f.c_flag() as u8;
        self.regs.f.set_h_flag(half_carry_add(self.regs.a, add));
        let (result, carried) = self.regs.a.overflowing_add(add);
        self.regs.a = result;
        self.regs.f.set_z_flag(self.regs.a == 0);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_c_flag(carried);
    }
    fn sub(&mut self, data: u8) {
        self.regs.f.set_h_flag(half_carry_sub(self.regs.a, data));
        let (result, carried) = self.regs.a.overflowing_sub(data);
        self.regs.a = result;
        self.regs.f.set_c_flag(carried);
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(true); 
    }
    fn sbc(&mut self, data: u8) {
        let data = self.regs.f.c_flag() as u8 + data;
        self.regs.f.set_h_flag(half_carry_sub(self.regs.a, data));
        let (result, carried) = self.regs.a.overflowing_sub(data);
        self.regs.a = result;
        self.regs.f.set_z_flag(self.regs.a==0);
        self.regs.f.set_n_flag(true);
        self.regs.f.set_c_flag(carried);
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
        let (result, carried) = r1.overflowing_add(r2);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_h_flag(half_carry_u16(r1, r2));
        self.regs.f.set_c_flag(carried);
        result
    }
    fn add_sp(&mut self, r2: i8) -> u16 {
        println!("this");
        let res = self.regs.sp.wrapping_add_signed(r2 as i16);
        // if it was an addition
        if r2 >= 0 {
            
        } else {

        }
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
    fn cf(&mut self, val: bool) {
        self.regs.f.set_c_flag(val);
        self.regs.f.set_n_flag(false);
        self.regs.f.set_h_flag(false);
    }
    fn cpl(&mut self) {
        self.regs.a = !self.regs.a;
        self.regs.f.set_n_flag(true);
        self.regs.f.set_h_flag(true);
    }
    /// logic flow opcodes
    /// handle the returns, jumps and calls in the assembly
    fn jr(&mut self, cc: bool) {
        // still have to load it so we update pc even if no jump
        let jump = self.next_byte() as i8;
        if cc {
            self.regs.jump_pc(jump);
        }
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
            None => self.next_word()
        };
        if !cc {
            // updating the stack pointer accordingly
            let next = self.regs.pc(); // instruction that we will jump back to on a return
            self.push(next);
            self.regs.set_pc(address);
        }
    }
    fn jp(&mut self, cc: bool, address: Option<u16>) {
        let address = match address {
            Some(a) => a,
            None => self.next_word(),
        };
        if cc {
            self.regs.set_pc(address);
        }
    }

    pub fn process_next(&mut self) {
        let opcode = self.next_byte();
        if !self.used.contains(&opcode) {
            self.used.push(opcode);
            println!("{:02X?}", self.used);
        }
        if opcode == 0xCB {
            self.process_prefixed();
            if self.scheduled_ime != self.ime {
                self.ime = self.scheduled_ime
            }
            return;
        }
        match opcode {
            0x00 => {}, // NOP
            0x01 => {let w=self.next_word(); self.regs.set_bc(w)} // LD BC, nn
            0x02 => self.write(self.regs.bc(), self.regs.a), // LD (BC), A
            0x03 => self.regs.set_bc(self.regs.bc().wrapping_add(1)), // INC BC
            0x04 => inc(&mut self.regs.b, &mut self.regs.f), // INC B
            0x05 => dec(&mut self.regs.b, &mut self.regs.f), // DEC B
            0x06 => self.regs.b = self.next_byte(), // LD B, n
            0x07 => rlc(&mut self.regs.a, &mut self.regs.f), // RLCA
            0x08 => self.write_u16(None, self.regs.sp), // LD (nn), SP
            0x09 => {let r=self.add_u16(self.regs.hl(), self.regs.bc()); self.regs.set_hl(r)}, // ADD HL, BC
            0x0A => self.regs.a = self.read(self.regs.bc()), // LD A, (BC)
            0x0B => self.regs.set_bc(self.regs.bc() - 1), // DEC BC
            0x0C => inc(&mut self.regs.c, &mut self.regs.f), // INC C
            0x0D => dec(&mut self.regs.c, &mut self.regs.f), // DEC C
            0x0E => self.regs.c = self.next_byte(), // LD C, n
            0x0F => rrc(&mut self.regs.a, &mut self.regs.f), // RRCA
            0x10 => {self.stopped = true; self.next_byte();}, // STOP n
            0x11 => {let w = self.next_word(); self.regs.set_de(w)} // LD DE, nn
            0x12 => self.write(self.regs.de(), self.regs.a), // LD (DE), A
            0x13 => self.regs.set_de(self.regs.de() + 1), // INC DE
            0x14 => inc(&mut self.regs.d, &mut self.regs.f), // INC D
            0x15 => dec(&mut self.regs.d, &mut self.regs.f), // DEC D
            0x16 => self.regs.d = self.next_byte(), // LD D, n
            0x17 => rl(&mut self.regs.a, &mut self.regs.f), // RLA
            0x18 => self.jr(true), // JR e
            0x19 => {let r= self.add_u16(self.regs.hl(), self.regs.de()); self.regs.set_hl(r)}, // ADD HL, DE
            0x1A => self.regs.a = self.read(self.regs.de()), // LD A, (DE)
            0x1B => self.regs.set_de(self.regs.de() - 1), // DEC DE
            0x1C => inc(&mut self.regs.e, &mut self.regs.f), // INC E
            0x1D => dec(&mut self.regs.e, &mut self.regs.f), // DEC E
            0x1E => self.regs.e = self.next_byte(), // LD E n
            0x1F => rr(&mut self.regs.a, &mut self.regs.f), // RRA
            0x20 => self.jr(!self.regs.f.z_flag()), // JR NZ, e
            0x21 => {let w = self.next_word(); self.regs.set_hl(w);} //LD HL, nn
            0x22 => {let hl=self.regs.hli(); self.write(hl, self.regs.a);} // LD (HL+), A
            0x23 => {self.regs.hli();}, // INC HL
            0x24 => inc(&mut self.regs.h, &mut self.regs.f), // INC H
            0x25 => dec(&mut self.regs.h, &mut self.regs.f), // DEC H
            0x26 => self.regs.h = self.next_byte(), // LD H, n
            0x27 => self.daa(), // DAA
            0x28 => self.jr(self.regs.f.z_flag()), // JR Z, e
            0x29 => {let r=self.add_u16(self.regs.hl(), self.regs.hl()); self.regs.set_hl(r)}, // ADD HL, HL
            0x2A => {let hl = self.regs.hli(); self.regs.a = self.read(hl)}, // LD A, (HL+)
            0x2B => self.regs.set_hl(self.regs.hl()-1), // DEC HL
            0x2C => inc(&mut self.regs.l, &mut self.regs.f), // INC L
            0x2D => dec(&mut self.regs.l, &mut self.regs.f), // DEC L
            0x2E => self.regs.l = self.next_byte(), // LD L, n
            0x2F => self.cpl(), // CPL
            0x30 => self.jr(!self.regs.f.c_flag()), // JR NC, e
            0x31 => self.regs.sp = combine_u8s(self.next_byte(), self.next_byte()), // LD SP, nn
            0x32 => {let hl = self.regs.hli(); self.write(hl, self.regs.a)} // LD (HL-), A
            0x33 => self.regs.sp += 1, // INC SP
            0x34 => {
                let mut data = self.read(self.regs.hl());
                inc(&mut data, &mut self.regs.f);
                self.write(self.regs.hl(), data);
            } // INC (HL)
            0x35 => {
                let mut data = self.read(self.regs.hl());
                dec(&mut data, &mut self.regs.f);
                self.write(self.regs.hl(), data);
            } // DEC (HL)
            0x36 => {let o = self.next_byte(); self.write(self.regs.hl(), o)} // LD (HL), n
            0x37 => self.cf(true), // SCF
            0x38 => self.jr(self.regs.f.c_flag()), // JR C, e
            0x39 => {let r= self.add_u16(self.regs.hl(), self.regs.sp); self.regs.set_hl(r)}, // ADD HL, SP
            0x3A => {let hl = self.regs.hld(); self.regs.a = self.read(hl)} // LD A, (HL-)
            0x3B => self.regs.sp -= 1, // DEC SP
            0x3C => inc(&mut self.regs.a, &mut self.regs.f), // INC A
            0x3D => dec(&mut self.regs.a, &mut self.regs.f), // DEC A
            0x3E => self.regs.a = self.next_byte(), // LD A, n
            0x3F => self.cf(!self.regs.f.c_flag()), // CCF
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
                    6 => self.read(self.regs.hl()),
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
                    6 => self.write(self.regs.hl(), src),
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
                    6 => self.read(self.regs.hl()),
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
            0xC2 => self.jp(!self.regs.f.z_flag(), None), // JP NZ, nn
            0xC3 => self.jp(true, None), // JP nn
            0xC4 => self.call(!self.regs.f.z_flag(), None), // CALL NZ nn
            0xC5 => self.push(self.regs.bc()), // PUSH BC
            0xC6 => {let o = self.next_byte(); self.add(o)} // ADD A, n
            0xC7 => self.call(true, Some(0x00)), // RST 00
            0xC8 => self.ret(self.regs.f.z_flag()), // RET Z
            0xC9 => self.ret(true), // RET
            0xCA => self.jp(self.regs.f.z_flag(), None), // JP Z, nn
            0xCC => self.call(self.regs.f.z_flag(), None), // CALL Z, nn
            0xCD => self.call(true, None), // CALL nn
            0xCE => {let o = self.next_byte(); self.adc(o)} // ADC A, n
            0xCF => self.call(true, Some(0x08)), // RST 08
            0xD0 => self.ret(!self.regs.f.c_flag()), // RET NC
            0xD1 => {let p = self.pop(); self.regs.set_de(p)} // POP DE 
            0xD2 => self.jp(!self.regs.f.c_flag(), None), // JP NC, nn
            0xD4 => self.call(!self.regs.f.c_flag(), None), // CALL NC, nn
            0xD5 => self.push(self.regs.de()), // PUSH DE
            0xD6 => {let o = self.next_byte(); self.sub(o)} // SUB n8
            0xD7 => self.call(true, Some(0x10)), // CALL 10
            0xD8 => self.ret(self.regs.f.c_flag()), // RET C
            0xD9 => {self.ret(true); self.scheduled_ime = true; return;}, // RETI
            0xDA => self.jp(self.regs.f.c_flag(), None), // JP C, nn
            0xDC => self.call(self.regs.f.c_flag(), None), // CALL C, nn
            0xDE => {let o = self.next_byte(); self.sbc(o)} // SBC A, n
            0xDF => self.call(true, Some(0x18)), // RST 18
            0xE0 => {let a = combine_u8s(self.next_byte(), 0xFF); self.write(a, self.regs.a)} // LDH (n), A
            0xE1 => {let o = self.pop(); self.regs.set_hl(o)} // POP HL
            0xE2 => {let a = combine_u8s(self.regs.c, 0xFF); self.write(a, self.regs.a)} // LD (C), A
            0xE5 => self.push(self.regs.hl()), // PUSH HL
            0xE6 => {let o = self.next_byte(); self.and(o)} // AND A, n
            0xE7 => self.call(true, Some(0x20)), // RST 20
            0xE8 => {let o = self.next_byte(); self.regs.sp = self.add_sp(o as i8)}, // ADD SP, e
            0xE9 => self.jp(true, Some(self.regs.hl())), // JP HL
            0xEA => {let w = self.next_word(); self.write(w, self.regs.a)} // LD (nn), A
            0xEE => {let o = self.next_byte(); self.xor(o)} // XOR A, n8
            0xEF => self.call(true, Some(0x28)), // RST 28
            0xF0 => {let a = combine_u8s(self.next_byte(), 0xFF); self.regs.a = self.read(a)}, // LDH A, (n8)
            0xF1 => {let p = self.pop(); self.regs.set_af(p)} // POP AF
            0xF2 => self.regs.a = self.read(combine_u8s(self.regs.c, 0xFF)), // LD A, (C)
            0xF3 => {self.scheduled_ime = false; return;}, // DI
            0xF5 => self.push(self.regs.af()), // PUSH AF
            0xF6 => {let o = self.next_byte(); self.or(o)} // OR A, n
            0xF7 => self.call(true, Some(0x30)), // RST 30
            0xF8 => {let o = self.next_byte(); let sp = self.add_sp(o as i8); self.regs.set_hl(sp)}, // LD HL, SP+e
            0xF9 => self.regs.sp = self.regs.hl(), // LD SP, HL
            0xFA => {let w = self.next_word(); self.regs.a = self.read(w)} // LD A, (n)
            0xFB => {self.scheduled_ime = true; return;}, // EI
            0xFE => {let o = self.next_byte(); self.cp(o)} // CP n8
            0xFF => self.call(true, Some(0x38)), // RST 38
            _ => panic!("unsupported opcode provided"),
        }
        if self.scheduled_ime != self.ime {
            self.ime = self.scheduled_ime;
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
                8..=15 => bit(dst, i-8, flags),
                16..=23 => res(dst, i-16),
                24..=31 => set(dst, i-24),
                _ => panic!("invalid instruction")
            }
        }
        let opcode = self.next_byte();
        let instruction = opcode / 8;
        let data_src = opcode % 8;

        // this works with memory so needs to be handled differently
        if data_src == 6 {
            let mut data = self.read(self.regs.hl());
            run_prefixed(&mut data, &mut self.regs.f, instruction);
            self.write(self.regs.hl(), data);
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