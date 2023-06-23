use super::*;
use inaccu6502::{Cpu, Memory};

pub struct System {
    cpu: Cpu,
    devices: Devices,
}

struct Devices {
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

impl Memory for Devices {
    fn read_byte(&self, address: u16) -> u8 {
        if address < 0x2000 {
            return self.ram[(address & 0x7FF) as usize];
        } else if address < 0x4000 {
            return self.ppu[(address & 0b111) as usize];
        } else if address < 0x4018 {
            return self.apu[(address - 0x4000) as usize];
        } else {
            // TODO: don't the hack
            let address = (address as usize) % self.cartridge.prg_data.len();
            return self.cartridge.prg_data[address];
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
        let mut result = System {
            cpu: Cpu::new(),
            devices: Devices {
                ram: [0; 2048],
                ppu: [0; 8],
                apu: [0; 24],
                cartridge,
            },
        };
        result.reset();
        result
    }
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.devices);
    }
    pub fn render(&mut self) -> [u32; NES_PIXEL_COUNT] {
        const CPU_STEPS_PER_SCANLINE: usize = 113;
        const CPU_STEPS_PER_VBLANK: usize = 2273;
        let mut result = [0xDEECAF; NES_PIXEL_COUNT];
        // Pretend to be in V-blank.
        // TODO: turn vblank flag on or something
        for _ in 0..CPU_STEPS_PER_VBLANK {
            self.cpu.step(&mut self.devices);
        }
        // TODO: turn vblank flag off
        for scanline in result.chunks_mut(NES_WIDTH) {
            // TODO: render a scanline
            for (i, pixel) in scanline.iter_mut().enumerate() {
                *pixel = (i as u32) * 69;
            }
            for _ in 0..CPU_STEPS_PER_SCANLINE {
                self.cpu.step(&mut self.devices);
            }
        }
        return result;
    }
}
