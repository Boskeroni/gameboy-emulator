use crate::{split_u16, combine_u8s};

///  Flag Register
///   ________________________________
///  | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
///  --------------------------------
/// | Z | N | H | C | 0 | 0 | 0 | 0 |
/// --------------------------------
pub struct Flags {
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}
impl Flags {
    fn new() -> Self {
        Self {
            z: false,
            n: false,
            h: false,
            c: false,
        }
    }

    pub fn set_z_flag(&mut self, data: bool) {
        self.z = data
    }
    pub fn set_n_flag(&mut self, data: bool) {
        self.n = data
    }
    pub fn set_h_flag(&mut self, data: bool) {
        self.h = data
    }
    pub fn set_c_flag(&mut self, data: bool) {
        self.c = data 
    }
    pub fn get_c_flag(&self) -> bool {
        self.c
    }
}

/// this struct should contain absolutely 0 logic of the program
/// it should simply allow for allocation of registers and reading
/// all logic should be handled in the cpu
/// it won't question anything just trust the data
pub struct Registers {
    pub a: u8,

    pub f: Flags,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    /// this is meant to be u16 but its easier if its just a usize
    /// because of array indexing
    pc: usize,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: Flags::new(),
            h: 0,
            l: 0,
            sp: 0xFFFE,
            pc: 0,
        }
    }



    // 16 bit register collectors
    pub fn set_bc(&mut self, data: u16) {
        (self.b, self.c) = split_u16(data);
    }
    pub fn set_de(&mut self, data: u16) {
        (self.d, self.e) = split_u16(data);
    }
    pub fn set_hl(&mut self, data: u16) {
        (self.h, self.l) = split_u16(data);
    }
    pub fn bc(&self) -> u16 {
        combine_u8s(self.b, self.c)
    }
    pub fn de(&self) -> u16 {
        combine_u8s(self.d, self.e)
    }
    pub fn hl(&self) -> u16 {
        combine_u8s(self.h, self.l)
    }

    pub fn pc(&mut self) -> usize {
        self.pc += 1;
        self.pc - 1
    }
}