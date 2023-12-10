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

use std::{env, cell::RefCell, rc::Rc, time::Instant, fs::File, io::Write};
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

// this number represents the number of cycles which each scanline will use up
const MAXCYCLES: usize = 453;

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
    let mut ppu = Ppu::new(memory.clone());

    // this cycle needs to run at below 16-milliseconds
    event_loop.run(move |event, elwt| {
        let mut new_frame_data: Vec<u8> = Vec::new();
        // handling the screen/inputs
        // rendering isnt done here as it wouldnt be able to follow the timings i would want it to
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        let mut debug_file = File::create("debug.gb").unwrap();
                        debug_file.write_all(&memory.borrow().memory).unwrap();
                        elwt.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        use winit::keyboard::{PhysicalKey::Code, KeyCode};

                        if let Code(e) = event.physical_key {
                            match e {
                                // this space will be used to handle all the inputs
                                // i honestly dont know how else to do this without the 
                                // many many inputs

                                // just as an experiment, q => quit
                                KeyCode::KeyQ => elwt.exit(),
                                _ => ()
                            }
                        }
                    }
                    _ => ()
                }
            },
            Event::AboutToWait => {
                // run a scanlines worth of processing per scanline
                for ly in 0..154 {
                    // update the ly value for the ppu
                    memory.borrow_mut().unchecked_write(0xFF44, ly);

                    let mut cycles = 0;
                    // handling the actual gameboy
                    // this handles everything for one scan-line.
                    while cycles < MAXCYCLES {
                        let new_cycles = cpu.process_next();
                        memory.borrow_mut().tick(new_cycles);
                        cycles += new_cycles as usize;
                    }
                    // get the new scanline ready
                    new_frame_data.append(&mut ppu.draw_scanline());
                }
                for (i, pixel) in pixels.frame_mut().chunks_exact_mut(4).enumerate() {
                    let new_pixel = pallete_to_rgba(new_frame_data[i]);
                    pixel[0] = new_pixel.0;
                    pixel[1] = new_pixel.1;
                    pixel[2] = new_pixel.2;
                    pixel[3] = 255;
                }

                // render the frame
                pixels.render().unwrap();
                window.request_redraw();
            },
            _ => ()
        }
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

fn pallete_to_rgba(i: u8) -> (u8, u8, u8) {
    match i {
        0 => (0x00, 0x00, 0x00),
        1 => (0x50, 0x50, 0x50),
        2 => (0xA0, 0xA0, 0xA0),
        3 => (0xFF, 0xFF, 0xFF),
        _ => panic!("invalid pallete index"),
    }
}