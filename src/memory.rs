use std::fs;

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
    pub memory: Vec<u8>,
    div: u16,
    overflow: bool,
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
        Self { memory, div: 0, overflow: false }
    }

    // this will just be oam dma
    pub fn dma(&mut self, pos: u8) {
        let src: usize = ((pos as u16) << 8) as usize;
        let dst: usize = 0xFE00;
        for i in 0..160 {
            self.memory[dst+i] = self.memory[src+i];
        }
    }

    /// the gameboy's memory is responsible for the timing of the machine
    /// all timing logic is handled within this function
    pub fn tick(&mut self, cycles: u8) {
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

    /// used as the internal way to writing to read only addresses
    /// the gameboy ROM never uses this only the emulator does
    pub fn unchecked_write(&mut self, address: u16, data: u8) {
        self.memory[address as usize] = data;
    }

    pub fn write(&mut self, address: u16, data: u8) {
        let address = address as usize;
        if address < 0x8000 {
            //panic!("cannot handle swapping yet, {address}");
        }

        // this address means dma is starting
        if address == 0xFF46 {
            self.dma(data)
        } else if address == 0xFF44 {
            return;
        }

        self.memory[address] = data;
        if address >= 0xC000 && address <= 0xDE00 {
            self.memory[address+0x2000] = data;
        } else if address >= 0xE000 && address <= 0xFE00 {
            self.memory[address-0x2000] = data;
        }
    }
    pub fn read(&self, address: u16) -> u8 {
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
                
                if lsb & (0b1000_0000 >> j) != 0 {
                    row_data |= 0b0000_0001;
                }
                if msb & (0b1000_0000) >> j != 0 {
                    row_data |= 0b0000_0010;
                }
            }
            tile_data[i] = row_data
        }
        tile_data.try_into().unwrap()
    }
    pub fn read_map(&self, index: u8) -> [u8; 1024] {
        if index > 1 {
            panic!("invalid map index: {index}")
        }
        let address = 0x9800 + ((index as usize)*1024);
        self.memory[address..(address+1024)].try_into().unwrap()
    }
}