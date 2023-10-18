use std::{rc::Rc, cell::RefCell};
use crate::memory::Memory;

pub struct Ppu {
    memory: Rc<RefCell<Memory>>,
    palette: [u8; 4],
    layers: [u8; 3],
}
impl Ppu {
    /// doesnt do anything with it. just gets it
    fn load_tile(&self, index: u8) -> u128 {
        self.memory.borrow().load_tile(index)
    }
    fn load_tile_map(&self, index: u8) -> [u8; 1024] {
        self.memory.borrow().load_map(index)
    }
    //fn load_object(&self, index: u8) -> [u8; 4] {
    //    //self.memory.borrow().load_object(index)
    //}
}