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
}
impl Ppu {
    pub fn new(mem: Rc<RefCell<Memory>>) -> Self {
        Self {
            memory: mem,
            scanline_buffer: Vec::new(),
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
        let ly = self.read_memory(PpuRegister::LY as u16);
        // checking each value in OAM
        for i in 0..40 {
            let potential = self.memory.borrow().read_oam(i);
            let height = {
                let mode = (self.read_memory(PpuRegister::LCDC as u16) & 0b0000_0100) != 0;
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
    pub fn draw_scanline(&mut self) -> Vec<u8> {
        let lcdc = self.read_memory(PpuRegister::LCDC as u16);
        // the screen is off
        if (lcdc & 0b1000_0000) == 0 {
            return  vec![0; 160];
        }
        self.oam_scan();
        

        let mut new_scanline = Vec::new();

        let scanline_y = self.read_memory(PpuRegister::LY as u16);
        let scroll_x = self.read_memory(PpuRegister::SCX as u16);
        let scroll_y = self.read_memory(PpuRegister::SCY as u16);

        let background_pallete = self.read_memory(PpuRegister::BGP as u16);
        let background_map = if lcdc & 0b0000_1000 != 0 { self.memory.borrow().read_map(0) } else { self.memory.borrow().read_map(1) };


        // implement the window later. TODO
        // 168 / 8 = 21
        for i in 0..21 {
            let background_pos_x = scroll_x.wrapping_add(i*8); // the pixel position in the background
            let background_pos_y = scroll_y.wrapping_add(scanline_y); 

            let background_tile_x = background_pos_x / 8; // the tile number from the left
            let background_tile_y = background_pos_y / 8;

            let background_tile_index = (background_tile_y as u16)*32 + background_tile_x as u16; // the tiles index in the map area
            let tile_index = background_map[background_tile_index as usize];
            let tile = self.memory.borrow().read_tile(tile_index); // the tile

            let tile_inner_row = background_pos_y%8;
            
            let row_data = tile[tile_inner_row as usize];
            // add the pallete to this
            for i in (0..8).rev() {
                let pallete_index = (row_data >> (i*2)) & 0b0000_0000_0000_0011;
                let real_color = (background_pallete >> (pallete_index*2)) & 0b0000_0011;
                new_scanline.push(real_color)
            }

            // now get the oam stuff

        }





        
        // handle the interrupt(s)
        if self.read_memory(0xFF44) == self.read_memory(0xFF45) {
            self.memory.borrow_mut().write(PpuRegister::STAT as u16, 0b0100_0000);
            let if_interrupt = self.read_memory(0xFF0F);
            self.memory.borrow_mut().write(0xFF0f, if_interrupt|0b0000_0010);
        }
        // i dont bother alerting the ppu modes as my emulator doesnt implement them normally
        new_scanline
    }
}