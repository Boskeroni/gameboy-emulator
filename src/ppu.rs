use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;

enum PpuRegister {
    SCY,
    SCX,
    WY=0xFF4A,
    WX=0xFF4B,
    LY,
    LCD=0xFF40,
    STAT=0xFF41,
}

pub struct Ppu {
    memory: Rc<RefCell<Memory>>,
    scanline_buffer: Vec<[u8; 4]>,
}
impl Ppu {
    pub fn new(mem: Rc<RefCell<Memory>>) -> Self {
        Self {
            memory: mem,
            scanline_buffer: Vec::new(),
        }
    }
    /// doesnt do anything with it. just gets it
    fn load_tile(&self, index: u8) -> u128 {
        self.memory.borrow().read_tile(index)
    }
    fn load_tile_map(&self, index: u8) -> [u8; 1024] {
        self.memory.borrow().read_map(index)
    }
    /// byte 0: Y position 0=>160
    /// byte 1: X position 0=>255
    /// byte 2: Tile index (using $8000 addressing)
    /// byte 3: Flags [7 => Obj priority, 6 => Y-flip, 5 => X-flip, 4 => Pallete]
    fn load_object(&self, index: u8) -> [u8; 4] {
        self.memory.borrow().read_object(index)
    }
    fn load_register(&self, reg: PpuRegister) -> u8 {
        use PpuRegister::*;
        let address = match reg {
            WY => 0xFF4A,
            WX => 0xFF4B,
            _ => todo!(),
        };
        self.memory.borrow().read(address)
    }

    /// this function should always take 80 T-cycles
    fn oam_scan(&mut self) {
        self.scanline_buffer.clear();
        let ly = self.load_register(PpuRegister::LY) + 16;
        // checking each value in OAM
        for i in 0..40 {
            let potential = self.load_object(i);
            let height = {
                let mode = (self.load_register(PpuRegister::LCD) & 0b0000_0100) != 0;
                8 + if mode {8} else {0}
            };


            if potential[1] == 0 { continue; } // dont display at 0
            if ly < potential[0] { continue; } // belongs to future scanline
            if ly >= potential[0] + height { continue; } // belonged to a past scanline
            self.scanline_buffer.push(potential);
            // once we get to 10 sprites. its reached its max
            if self.scanline_buffer.len() == 10 {
               break;
            }
        }
    }
    /// this transfers pixels to the LCD. The timings for the function can change though
    fn drawing(&self) {
        todo!();
    }
    fn h_blank(&self) {
        todo!();
    }
    fn v_blank(&self) {
        todo!();
    }

    pub fn render(&mut self) {
        todo!();
    }
}