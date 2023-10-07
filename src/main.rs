mod cpu;
mod memory;
mod registers;

use std::{env, cell::RefCell, rc::Rc};
use cpu::Cpu;
use memory::Memory;

/// little endian reading
/// the second parameter is the upper byte of the u16
/// the first parameter is the lower byte of the u16
pub fn combine_u8s(a: u8, b: u8) -> u16 {
    ((b as u16) << 8) + a as u16
}
pub fn split_u16(a: u16) -> (u8, u8) {
    ((a & 0xFF) as u8, (a >> 8) as u8)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // no file path provided
    if args.len() == 1 {
        panic!("no file path was provided");
    }
    let rom_path = &args[1];
    println!("{rom_path}");
    let rom = match std::fs::read(rom_path) {
        Err(_) => panic!("invalid file provided"),
        Ok(f) => f,
    };
    
    let memory = Rc::new(RefCell::new(Memory::new()));

    let mut cpu = Cpu::new(memory.clone(), rom);
    loop {
        cpu.process_next();
        println!("");
        // used for outputs during blarggs tests and since thatll be
        // all the gameboy roms ill be running for a while no point
        // in it being a seperate function. itll be easily deletable later
        if memory.borrow().load(0xFF02) == 0x81 {
            let c = memory.borrow().load(0xFF01) as char;
            print!("{c}");
            memory.borrow_mut().write_u8(0xFF02, 0);
        }
    }
}