use super::*;
use inaccu6502::{Cpu, Memory};

#[derive(Default)]
pub struct Controller {
    pub button_a: bool,
    pub button_b: bool,
    pub button_select: bool,
    pub button_start: bool,
    pub button_up: bool,
    pub button_down: bool,
    pub button_left: bool,
    pub button_right: bool,
    latch_state: bool,
    captured_byte: u8,
}

const BUTTON_A: u8 = /*     */ 0b0000_0001;
const BUTTON_B: u8 = /*     */ 0b0000_0010;
const BUTTON_SELECT: u8 = /**/ 0b0000_0100;
const BUTTON_START: u8 = /* */ 0b0000_1000;
const BUTTON_UP: u8 = /*    */ 0b0001_0000;
const BUTTON_DOWN: u8 = /*  */ 0b0010_0000;
const BUTTON_LEFT: u8 = /*  */ 0b0100_0000;
const BUTTON_RIGHT: u8 = /* */ 0b1000_0000;

impl Controller {
    fn capture_byte(&self) -> u8 {
        let mut result = 0;
        if self.button_a {
            result |= BUTTON_A;
        }
        if self.button_b {
            result |= BUTTON_B;
        }
        if self.button_select {
            result |= BUTTON_SELECT;
        }
        if self.button_start {
            result |= BUTTON_START;
        }
        if self.button_up {
            result |= BUTTON_UP;
        }
        if self.button_down {
            result |= BUTTON_DOWN;
        }
        if self.button_left {
            result |= BUTTON_LEFT;
        }
        if self.button_right {
            result |= BUTTON_RIGHT;
        }
        return result;
    }
    fn set_latch_state(&mut self, state: bool) {
        self.latch_state = state;
        if self.latch_state {
            self.captured_byte = self.capture_byte();
        }
    }
    fn perform_read(&mut self) -> u8 {
        if self.latch_state {
            // If the latch is currently on, we can't shift bits out. Just
            // keep capturing and capturing and...
            self.captured_byte = self.capture_byte();
        }
        let result = self.captured_byte & 1;
        if !self.latch_state {
            // If the latch is off, we shift one bit out.
            self.captured_byte = (self.captured_byte >> 1) | 0x80;
        }
        return result;
    }
}

pub struct System {
    cpu: Cpu,
    devices: Devices,
}

struct Devices {
    ram: [u8; WORK_RAM_SIZE],
    /// Picture Processing Unit
    /// TODO: PPU registers
    ppu: [u8; 8],
    /// Audio Processing Unit
    /// TODO: APU and IO registers
    apu: [u8; 24],
    cartridge: Cartridge,
    vblank_status_flag: bool,
    pub controllers: [Controller; 2],
}

// 0x2456
// vvv
// 0010 0100 1001 1010
// 000: WRAM
//    x xAAA AAAA AAAA
// 001: PPU
//    x xxxx xxxx xAAA

impl Memory for Devices {
    fn read_byte(&mut self, address: u16) -> u8 {
        if address < 0x2000 {
            self.ram[(address & 0x7FF) as usize]
        } else if address < 0x4000 {
            match address {
                0x2002 => {
                    let mut result = 0;
                    // Sprite Overflow flag. The real hardware is buggy as
                    // hell. For now, we won't try to implement it.
                    if false {
                        result |= 0x20;
                    }
                    // Sprite 0 Hit flag. Not implemented YET, but we do plan
                    // to implement it eventually.
                    if false {
                        result |= 0x40;
                    }
                    // Vertical Blank flag.
                    if self.vblank_status_flag {
                        result |= 0x80;
                        self.vblank_status_flag = false;
                    }
                    result
                }
                _ => self.ppu[(address & 0b111) as usize],
            }
        } else if address < 0x4018 {
            match address {
                0x4016 => self.controllers[0].perform_read(),
                0x4017 => self.controllers[1].perform_read(),
                _ => self.apu[(address - 0x4000) as usize],
            }
        } else {
            // TODO: don't the hack
            let address = (address as usize) % self.cartridge.prg_data.len();
            self.cartridge.prg_data[address]
        }
    }
    fn write_byte(&mut self, address: u16, data: u8) {
        if address < 0x2000 {
            self.ram[(address & 0x7FF) as usize] = data;
        } else if address < 0x4000 {
            match address {
                0x2002 => warn!("ROM wrote {data:02X} to PPUSTATUS register"),
                _ => self.ppu[(address & 0b111) as usize] = data,
            }
        } else if address < 0x4018 {
            match address {
                0x4016 => {
                    self.controllers[0].set_latch_state(data & 1 != 0);
                    self.controllers[1].set_latch_state(data & 1 != 0);
                }
                0x4017 => warn!("What is this rom doing, writing to 0x4017???"),
                _ => self.apu[(address - 0x4000) as usize] = data,
            }
        } else {
            warn!(
                "Attempted write to cartridge: {:04X} <-- {:02X}",
                address, data
            );
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
                // Any array of things that implement Default also implements
                // Default, so we can Default our Default to Default the
                // defaults. Nicer than [Controller::new() * n]
                controllers: Default::default(),
                vblank_status_flag: true,
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
        self.devices.vblank_status_flag = true; // vblank flag ON
        for _ in 0..CPU_STEPS_PER_VBLANK {
            self.cpu.step(&mut self.devices);
        }
        self.devices.vblank_status_flag = false; // vblank flag OFF
        for (y, scanline) in result.chunks_mut(NES_WIDTH).enumerate() {
            // TODO: render a scanline
            for (x, pixel) in scanline.iter_mut().enumerate() {
                *pixel = (x as u32) * 69 + (y as u32) * 420;
            }
            for _ in 0..CPU_STEPS_PER_SCANLINE {
                self.cpu.step(&mut self.devices);
            }
        }
        return result;
    }
    pub fn show_cpu_state(&self) -> String {
        format!("CPU: {:?}", self.cpu)
    }
    pub fn get_work_memory_byte(&self, address: u16) -> u8 {
        let address = address as usize;
        assert!(address < WORK_RAM_SIZE, "Invalid RAM address {address:04X}");
        return self.devices.ram[address];
    }
    pub fn get_controllers(&self) -> &[Controller] {
        return &self.devices.controllers;
    }
    pub fn get_controllers_mut(&mut self) -> &mut [Controller] {
        return &mut self.devices.controllers;
    }
}
