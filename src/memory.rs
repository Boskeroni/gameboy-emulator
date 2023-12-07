use crate::combine_u8s;

/// just makes it more clear what my magic numbers are
/// also since the timings are handled by the memory instead of the CPU
#[repr(usize)]
enum TimingRegisters {
    DIV=0xFF04,
    TIMA=0xFF05,
    TMA=0xFF06,
    TAC=0xFF07,
}

pub struct Memory {
    memory: Vec<u8>,
    div: u16,
    overflow: bool,
    dma_i: u16,
    active_dma: bool
}

impl Memory {
    pub fn new(rom: Vec<u8>) -> Self {
        // for now all the roms will only be up to 0x8000 in length
        // so we can just extend till we reach the quota
        let mut memory = rom;
        if memory.len() > 0x8000 {
            panic!("not going to handle these yet")
        }
        let padding_amount = 65536 - memory.len();
        let padding_vec = vec![0; padding_amount];
        memory.extend(padding_vec);
        Self { memory, div: 0, overflow: false, dma_i: 0, active_dma: false }
    }

    // this will just be oam dma
    pub fn dma(&mut self) {
        let src = (self.memory[0xFF46] as u16) << 8 + self.dma_i;
        let dst = 0xFE00 + self.dma_i; // since it is just oam dma
        
        self.memory[dst as usize] = self.memory[src as usize];
        self.dma_i += 1;
        if self.dma_i == 160 {
            // the dma transferred has ended
            self.active_dma = false;
        }
    }

    /// the gameboy's memory is responsible for the timing of the machine
    /// all timing logic is handled within this function
    pub fn tick(&mut self, cycles: u8) {
        if self.active_dma {
            for _ in 0..cycles/4 {
                self.dma();
            }
        }

        // due to timings, the overflow logic can only be handled after the next instruction 
        if self.overflow {
            // set tima to tma and call an interrupt
            self.memory[TimingRegisters::TIMA as usize] = self.memory[TimingRegisters::TMA as usize];
            self.memory[0xFF06] &= 0b0000_0100;   
        }
        
        let tac = self.memory[TimingRegisters::TAC as usize];

        let timing_bits = tac & 0b0000_0011;
        let rate = if timing_bits == 0 { 9 } else { 1+(2*timing_bits) };
        // a 1 means that it is active
        if (tac & 0b0000_0100) != 0 &&  self.div & (1 << rate) > self.div.wrapping_add(cycles as u16) & (1 << rate) {
            let (new, carry) = self.memory[TimingRegisters::TIMA as usize].overflowing_add(1);
            self.memory[TimingRegisters::TIMA as usize] = new;
            self.overflow = carry;
        }
        
        // this just handles an edge case
        // actually increment
        self.div = self.div.wrapping_add(cycles as u16);

        // only map the top 8 bits to the memory
        self.memory[TimingRegisters::DIV as usize] = (self.div >> 8) as u8;
    }

    pub fn write(&mut self, address: u16, data: u8) {
        if self.active_dma {
            return;
        }

        let address = address as usize;
        if address < 0x8000 {
            panic!("cannot handle swapping yet");
        }

        // edge case with the div register
        if address == TimingRegisters::DIV as usize {
            self.memory[address] = 0;
            return;
        }
        // this address means dma is starting
        else if address == 0xFF46 {
            self.active_dma = true;
        }

        self.memory[address] = data;
        if address >= 0xC000 && address <= 0xDE00 {
            self.memory[address+0x2000] = data;
        } else if address >= 0xE000 && address <= 0xFE00 {
            self.memory[address-0x2000] = data;
        }
    }
    pub fn read(&self, address: u16) -> u8 {
        if self.active_dma {
            let src = (self.memory[0xFF46] as u16) << 8 + self.dma_i;
            return self.memory[src as usize];
        }
        self.memory[address as usize]
    }



    /// these are all the functions for collecting pixel data for the ppu
    pub fn read_oam(&self, index: u8) -> [u8; 4] {
        if index >= 40 {
            panic!("invalid oam entry asked for");
        }
        let src = (0xFE00 + (index as u16) * 4) as usize;
        return self.memory[src..src+4].try_into().unwrap()
    }
    /// this returns [u16; 8] rather than [u8; 16] as each line of sprite data
    /// is stored within the u16s rather than two u8s
    pub fn read_tile(&self, index: u8) -> [u16; 8] {
        // just going to assume its 8000 addressing
        let src = 0x8000 + (index as usize)*16;
        let mut tile_data = vec![0; 8];
        for i in 0..8 {
            let mut row_data: u16 = 0;
            let lsb = self.memory[src+i*2];
            let msb = self.memory[src+i*2+1];
            for j in 0..8 {
                row_data = row_data << 2;
                let new_data = ((lsb & (0b1000_0000>>j)) + (msb & (0b1000_0000>>j)) * 2) as u16;
                row_data += new_data;
            }

            tile_data[i] = row_data
        }
        tile_data.try_into().unwrap()
    }
}