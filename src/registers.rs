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
    fn from_u8(data: u8) -> Self {
        Self {
            z: data & 0b1000_0000 != 0,
            n: data & 0b0100_0000 != 0,
            h: data & 0b0010_0000 != 0,
            c: data & 0b0001_0000 != 0,
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
    pub fn get_z_flag(&self) -> bool {
        self.z
    }

    fn as_u8(&self) -> u8 {
        (self.z as u8) << 7 | (self.n as u8) << 6 | (self.h as u8) << 5 | (self.c as u8) << 4
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
            pc: 0x100,
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
    pub fn set_af(&mut self, data: u16) {
        let (a, f) = split_u16(data);
        self.a = a;
        self.f = Flags::from_u8(f);
    }

    pub fn bc(&self) -> u16 {
        combine_u8s(self.c, self.b)
    }
    pub fn de(&self) -> u16 {
        combine_u8s(self.e, self.d)
    }
    pub fn hl(&self) -> u16 {
        combine_u8s(self.l, self.h)
    }
    pub fn af(&self) -> u16 {
        combine_u8s(self.f.as_u8(), self.a)
    }

    pub fn pc(&mut self) -> usize {
        self.pc += 1;
        print!("pc: {}", self.pc);
        self.pc-1
    }
    pub fn set_pc(&mut self, val: u16) {
        self.pc = val as usize;
    }
    pub fn jump_pc(&mut self, val: i8) {
        if val >= 0 {
            self.pc += val as usize;
        } else {
            self.pc -= val.abs() as usize;
        }
    }
}