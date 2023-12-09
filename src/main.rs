#![allow(unused)]
#![allow(unreachable_code)]

mod cpu;
mod memory;
mod registers;
mod ppu;
mod opcodes;

use cpu::Cpu;
use ppu::Ppu;
use memory::Memory;

use std::{env, cell::RefCell, rc::Rc};
use pixels::{SurfaceTexture, Pixels, Error};
use winit::{
    dpi::LogicalSize, 
    event::{Event, WindowEvent},
    event_loop::{EventLoop, ControlFlow}, 
    window::WindowBuilder
};
use winit_input_helper::WinitInputHelper;

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
const MAXCYCLES: usize = 69905/60;
const SCREEN_HEIGHT: u32 = 160;
const SCREEN_WIDTH: u32 = 144;

fn main() {
    // setting up the window
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64);
        let scaled_size = LogicalSize::new(SCREEN_WIDTH as f64 * 3.0, SCREEN_HEIGHT as f64 * 3.0);
        WindowBuilder::new()
            .with_title("gameboy emulator")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).unwrap()
    };

    let rom = get_rom();

    // all the pillars of a gameboy emulator
    let memory = Rc::new(RefCell::new(Memory::new(rom)));
    let mut cpu = Cpu::new(memory.clone());
    let mut _ppu = Ppu::new(memory.clone());

    let mut cycles: usize = 0;
    let mut new_cycles: u8 = 0;
    let mut redraws = 0;

    event_loop.run(move |event, elwt| {
        // handling the screen/inputs
        // rendering isnt done here as it wouldnt be able to follow the timings i would want it to
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                elwt.exit();
            }
            Event::UserEvent(event) => {
                println!("{event:?}");
            }
            _ => ()
        }
        // handling the actual gameboy
        while cycles < MAXCYCLES {
            new_cycles = cpu.process_next();
            memory.borrow_mut().tick(new_cycles);
    
            cycles += new_cycles as usize;
            println!("{cycles}, {redraws}");
            if memory.borrow().read(0xFF02) == 0x81 {
                let c = memory.borrow().read(0xFF01) as char;
                print!("{c}");
                memory.borrow_mut().write(0xFF02, 0);
            }
        }
        cycles = 0;
    });
}

fn get_rom() -> Vec<u8> {
    let args: Vec<String> = env::args().collect();

    // no file path provided
    if args.len() == 1 {
        panic!("no file path was provided");
    }
    let rom_path = &args[1];
    match std::fs::read(rom_path) {
        Err(_) => panic!("invalid file provided"),
        Ok(f) => f,
    }
}