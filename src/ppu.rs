use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;

enum PpuRegister {
    LCDC=0xFF40,
    STAT=0xFF41,
    SCY=0xFF42,
    SCX=0xFF43,
    LY=0xFF44,
    LYC=0xFF45,
    BGP=0xFF47,
    OBP0=0xFF48,
    OBP1=0xFF49,
    WY=0xFF4A, // y position of the top border of the window (0 at top)
    WX=0xFF4B, // x position of the left broder of the window (7 at left)
}

pub struct Ppu {
    memory: Rc<RefCell<Memory>>,
    scanline_buffer: Vec<[u8; 4]>,
    background_fifo: u32,
    sprite_fifo: u32,
}
impl Ppu {
    pub fn new(mem: Rc<RefCell<Memory>>) -> Self {
        Self {
            memory: mem,
            scanline_buffer: Vec::new(),
            background_fifo: 0,
            sprite_fifo: 0,
        }
    }

    /// once again just simplifies reading memory in other functions
    /// easier to call `self.read_memory(address)` rather than `self.memory.borrow().read(address)`
    fn read_memory(&self, address: u16) -> u8 {
        self.memory.borrow().read(address)
    }

    /// this function should always take 80 T-cycles
    fn oam_scan(&mut self) {
        self.scanline_buffer.clear();
        let ly = self.read_memory(PpuRegister::LY as u16) + 16;
        // checking each value in OAM
        for i in 0..40 {
            let potential = self.memory.borrow().read_oam(i);
            let height = {
                let mode = (self.read_memory(PpuRegister::LCDC as u16) & 0b0000_0100) != 0;
                8 + if mode {8} else {0}
            };

            if potential[1] == 0 { continue; } // dont display at 0
            if ly + 16 < potential[0] { continue; } // belongs to future scanline
            if ly + 16 >= potential[0] + height { continue; } // belonged to a past scanline
            self.scanline_buffer.push(potential);
            // once we get to 10 sprites. its reached its max
            if self.scanline_buffer.len() == 10 {
               break;
            }
        }
    }
    /// this transfers pixels to the LCD. The timings for the function can change though
    pub fn draw_scanline(&mut self) -> [u8; 144] {
        todo!();
    }
}