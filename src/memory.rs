use crate::split_u16;

pub struct Memory {
    memory: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        let size: usize = 16;
        Self { memory: Vec::with_capacity(size.pow(4))}
    }

    pub fn write_u8(&mut self, address: u16, data: u8) {
        self.memory[address as usize] = data;
    }
    pub fn write_u16(&mut self, address: u16, data: u16) {
        let (store1, store2) = split_u16(data);
        self.memory[address as usize] = store1;
        self.memory[(address+1) as usize] = store2;
    }
    pub fn load(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }
}