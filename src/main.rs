mod cpu;
mod memory;
mod registers;

use std::{env, cell::RefCell, rc::Rc};
use cpu::Cpu;
use memory::Memory;

pub fn combine_u8s(a: u8, b: u8) -> u16 {
    ((a as u16) << 8) + b as u16
}
pub fn split_u16(a: u16) -> (u8, u8) {
    ((a >> 8) as u8, a as u8)
}

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

    let memory = Rc::new(RefCell::new(Memory::new()));

    let mut cpu = Cpu::new(memory, rom);
    loop {
        cpu.process_next();
    }
}