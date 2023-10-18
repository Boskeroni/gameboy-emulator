use crate::split_u16;

pub struct Memory {
    memory: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { memory: vec![0; 65536]}
    }

    pub fn write_u8(&mut self, address: u16, data: u8) {
        self.memory[address as usize] = data;
    }
    pub fn write_u16(&mut self, address: u16, data: u16) {
        let (store1, store2) = split_u16(data);
        self.memory[address as usize] = store1;
        self.memory[(address+1) as usize] = store2;
    }
    pub fn read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }
    pub fn read_mut(&mut self, address: u16) -> &mut u8 {
        self.memory.get_mut(address as usize).unwrap()
    }
    /// loading calls for the ppu
    /// returns a u128 as it is more memory efficient
    pub fn load_tile(&self, index: u8) -> u128 {
        // using $8000 addressing, idk why the other one exists
        let addressing = 0x8000 + (index * 16) as usize;
        let mut tile_data: u128 = 0;
        for i in 0..8 {
            let least_sig = self.memory[addressing+i];
            let most_sig = self.memory[addressing+i+1];
            for j in 0..8 {
                // shifts the data by two so that we can add the next 
                // tile onto it. Just kinda forced it to return a u128
                // since it seems to be the most memory efficient option
                // will probably change in benchmarking
                tile_data = tile_data << 2;
                tile_data += ((least_sig >> j & 1) + (most_sig >> j & 1)*2) as u128; 
            }
        }
        tile_data
    }
    /// the index will only every be a 0 or 1 
    /// so there is little chance of it erroring unless the gameboy file
    /// is flawed
    pub fn load_map(&self, index: u8) -> [u8; 1024] {
        if index > 1 {
            panic!("invalid map index you dumbass {index}");
        }
        let address = 0x9800 + ((index as usize)*1024);
        self.memory[address..(address+1024)].try_into().unwrap()
    }
}