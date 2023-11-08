#![allow(dead_code)]

mod cpu;
mod memory;
mod registers;
mod ppu;
mod opcodes;

use std::{env, cell::RefCell, rc::Rc};
use cpu::Cpu;
use memory::Memory;
use ppu::Ppu;

/// little endian reading;
/// 
/// the first number parsed will be the lower byte and the 
/// second will be the upper byte.
pub fn combine_u8s(lsb: u8, msb: u8) -> u16 {
    ((msb as u16) << 8) + lsb as u16
}
/// the upper byte is returned first. the lower byte is
/// returned secondly.
pub fn split_u16(a: u16) -> (u8, u8) {
    ((a >> 8) as u8, (a & 0xFF) as u8)
}
///https://www.reddit.com/r/EmuDev/comments/4o2t6k/how_do_you_emulate_specific_cpu_speeds/
const MAXCYCLES: usize = 69905;

fn main() {
    let args: Vec<String> = env::args().collect();

    // no file path provided
    if args.len() == 1 {
        panic!("no file path was provided");
    }
    let rom_path = &args[1];
    let rom = match std::fs::read(rom_path) {
        Err(_) => panic!("invalid file provided"),
        Ok(f) => f,
    };
    
    // all the pillars of a gameboy emulator
    let memory = Rc::new(RefCell::new(Memory::new(rom)));
    let mut cpu = Cpu::new(memory.clone());
    let mut _ppu = Ppu::new(memory.clone());

    let mut cycles: usize = 0;
    loop {
        cycles += cpu.process_next() as usize;
        if cycles >= MAXCYCLES {
            //ppu.render();
            cycles = 0;
        }
        // used for outputs during blarggs tests and since thatll be
        // all the gameboy roms ill be running for a while no point
        // in it being a seperate function. itll be easily deletable later
        if memory.borrow().read(0xFF02) == 0x81 {
            let c = memory.borrow().read(0xFF01) as char;
            print!("{c}");
            memory.borrow_mut().write(0xFF02, 0);
        }
    }
}