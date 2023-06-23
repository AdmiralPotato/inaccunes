use super::*;
use inaccu6502::{Cpu, Memory};

pub struct System {
    cpu: Cpu,
    ram: [u8; 2048],
    /// Picture Processing Unit
    /// TODO: PPU registers
    ppu: [u8; 8],
    /// Audio Processing Unit
    /// TODO: APU and IO registers
    apu: [u8; 24],
    // TODO: add gamepad state
    cartridge: Cartridge,
}

// 0x2456
// vvv
// 0010 0100 1001 1010
// 000: WRAM
//    x xAAA AAAA AAAA
// 001: PPU
//    x xxxx xxxx xAAA

impl Memory for System {
    fn read_byte(&self, address: u16) -> u8 {
        if address < 0x2000 {
            return self.ram[(address & 0x7FF) as usize];
        } else if address < 0x4000 {
            return self.ppu[(address & 0b111) as usize];
        } else if address < 0x4018 {
            return self.apu[(address - 0x4000) as usize];
        } else {
            todo!("cartridge space read {:04X}", address);
        }
    }
    fn write_byte(&mut self, address: u16, data: u8) {
        if address < 0x2000 {
            self.ram[(address & 0x7FF) as usize] = data;
        } else if address < 0x4000 {
            self.ppu[(address & 0b111) as usize] = data;
        } else if address < 0x4018 {
            self.apu[(address - 0x4000) as usize] = data;
        } else {
            panic!("Do not write to cartridge!! {:04X}", address);
        }
    }
}

impl System {
    pub fn new(cartridge: Cartridge) -> System {
        return System {
            cpu: Cpu::new(),
            ram: [0; 2048],
            ppu: [0; 8],
            apu: [0; 24],
            cartridge,
        };
    }
    pub fn render(&mut self) -> [u32; NES_PIXEL_COUNT] {
        let mut result = [0xDEECAF; NES_PIXEL_COUNT];
        result[(NES_PIXEL_COUNT / 2) + (NES_WIDTH / 2)] = 0xff0000;
        return result;
    }
}
