use std::fmt::{Debug, Formatter, Result as FmtResult};

use super::*;

mod ppu;
use inaccu6502::{Cpu, Memory};
use ppu::*;

const TILE_BYTES: usize = 16;
const MAX_SPRITES_PER_SCANLINE: usize = 8;

const BUTTON_A: u8 = /*     */ 0b0000_0001;
const BUTTON_B: u8 = /*     */ 0b0000_0010;
const BUTTON_SELECT: u8 = /**/ 0b0000_0100;
const BUTTON_START: u8 = /* */ 0b0000_1000;
const BUTTON_UP: u8 = /*    */ 0b0001_0000;
const BUTTON_DOWN: u8 = /*  */ 0b0010_0000;
const BUTTON_LEFT: u8 = /*  */ 0b0100_0000;
const BUTTON_RIGHT: u8 = /* */ 0b1000_0000;

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

impl Debug for Controller {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{a}{b}{e}{s}{u}{d}{l}{r}",
            a = if self.button_a { 'A' } else { 'a' },
            b = if self.button_b { 'B' } else { 'b' },
            e = if self.button_select { 'E' } else { 'e' },
            s = if self.button_start { 'S' } else { 's' },
            u = if self.button_up { 'U' } else { 'u' },
            d = if self.button_down { 'D' } else { 'd' },
            l = if self.button_left { 'L' } else { 'l' },
            r = if self.button_right { 'R' } else { 'r' },
        )
    }
}

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

pub struct Devices {
    ram: [u8; WORK_RAM_SIZE],
    /// Picture Processing Unit
    /// TODO: PPU registers
    ppu: PPU,
    /// Audio Processing Unit
    /// TODO: APU and IO registers
    apu: [u8; 24],
    cartridge: Cartridge,
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
    fn read_byte(&mut self, _cpu: &mut Cpu, address: u16) -> u8 {
        if address < 0x2000 {
            self.ram[(address & 0x7FF) as usize]
        } else if address < 0x4000 {
            self.ppu.perform_register_read(&self.cartridge, address)
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
    fn write_byte(&mut self, cpu: &mut Cpu, address: u16, data: u8) {
        if address < 0x2000 {
            self.ram[(address & 0x7FF) as usize] = data;
        } else if address < 0x4000 {
            self.ppu
                .perform_register_write(cpu, &mut self.cartridge, address, data)
        } else if address < 0x4018 {
            match address {
                0x4014 => {
                    // OAM DMA!!!!
                    let page_to_read = data;
                    let start_address = u16::from_be_bytes([page_to_read, 0]);
                    for src_address in start_address..=start_address + 255 {
                        let oam_data = self.read_byte(cpu, src_address);
                        self.write_byte(cpu, 0x2004, oam_data);
                    }
                }
                0x4016 => {
                    self.controllers[0].set_latch_state(data & 1 != 0);
                    self.controllers[1].set_latch_state(data & 1 != 0);
                }
                0x4017 => {
                    // warn!("What is this rom doing, writing to 0x4017???")
                }
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

impl Devices {
    pub fn get_ppu(&self) -> &PPU {
        &self.ppu
    }
    pub fn get_ram(&self) -> &[u8; WORK_RAM_SIZE] {
        &self.ram
    }
}

struct Sprite {
    x: usize,
    y: usize,
    tile_address: u16,
    palette: usize,
    is_behind_background: bool,
    flip_horizontal: bool,
    flip_vertical: bool,
}

impl Sprite {
    pub fn from_oam_data(
        sprites_are_8x16: bool,
        sprite_tiles_are_in_upper_half: bool,
        oam_data: &[u8],
    ) -> Sprite {
        debug_assert_eq!(oam_data.len(), 4);
        let y = oam_data[0] as usize + 1;
        let tile_address = if sprites_are_8x16 {
            let tile_number = oam_data[1] & 0b1111_1110;
            let tile_offset = tile_number as u16 * TILE_BYTES as u16;
            if oam_data[1] & 1 != 0 {
                0x1000 + tile_offset
            } else {
                0x0000 + tile_offset
            }
        } else if sprite_tiles_are_in_upper_half {
            let tile_offset = oam_data[1] as u16 * TILE_BYTES as u16;
            0x1000 + tile_offset
        } else {
            let tile_offset = oam_data[1] as u16 * TILE_BYTES as u16;
            0x0000 + tile_offset
        };
        let attributes = oam_data[2];
        let palette = (attributes as usize & 0b0000_0011) + 4;
        let is_behind_background = (attributes & 0b0010_0000) != 0;
        let flip_horizontal = (attributes & 0b0100_0000) != 0;
        let flip_vertical = (attributes & 0b1000_0000) != 0;
        let x = oam_data[3] as usize;
        Sprite {
            x,
            y,
            tile_address,
            palette,
            is_behind_background,
            flip_horizontal,
            flip_vertical,
        }
    }
    fn is_visible_on_scanline(&self, sprites_are_8x16: bool, y: usize) -> bool {
        let size = if sprites_are_8x16 { 16 } else { 8 };
        (self.y..self.y + size).contains(&y)
    }
    fn get_pixel_for_xy(
        &self,
        cartridge: &Cartridge,
        sprites_are_8x16: bool,
        x: usize,
        y: usize,
    ) -> Option<(u8, usize, bool)> {
        if (self.x..self.x + 8).contains(&x) {
            let sprite_palette = self.palette;
            let sprite_is_behind_background = self.is_behind_background;
            let x_within_sprite = x - self.x;
            let x_within_sprite = if self.flip_horizontal {
                x_within_sprite
            } else {
                7 - x_within_sprite
            };
            let y_within_sprite = y - self.y;
            let y_within_sprite = if self.flip_vertical {
                if sprites_are_8x16 {
                    15 - y_within_sprite
                } else {
                    7 - y_within_sprite
                }
            } else {
                y_within_sprite
            };
            let y_within_sprite = if y_within_sprite >= 8 {
                // (this can only happen with 8x16 sprites)
                y_within_sprite + 8 // skip to the next tile number
            } else {
                y_within_sprite
            };
            let low_byte = cartridge.chr_data[self.tile_address as usize + y_within_sprite];
            let high_byte = cartridge.chr_data[self.tile_address as usize + y_within_sprite + 8];
            let mask = 1 << x_within_sprite;
            let low_masked = (low_byte & mask) >> x_within_sprite;
            let high_masked = (high_byte & mask) >> x_within_sprite << 1;
            let sprite_color = low_masked | high_masked;
            if sprite_color == 0 {
                // If the sprite is transparent, another sprite could still
                // be here
                None
            } else {
                Some((sprite_color, sprite_palette, sprite_is_behind_background))
            }
        } else {
            None
        }
    }
}

impl System {
    pub fn new(cartridge: Cartridge) -> System {
        let mut result = System {
            cpu: Cpu::new(),
            devices: Devices {
                ram: [0; 2048],
                ppu: PPU::new(),
                apu: [0; 24],
                cartridge,
                // Any array of things that implement Default also implements
                // Default, so we can Default our Default to Default the
                // defaults. Nicer than [Controller::new() * n]
                controllers: Default::default(),
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
        let mut result = [0xBEECAF; NES_PIXEL_COUNT];
        // Pretend to be in V-blank.
        self.devices.ppu.vblank_start(&mut self.cpu); // vblank flag ON
        for _ in 0..CPU_STEPS_PER_VBLANK {
            self.cpu.step(&mut self.devices);
        }
        self.devices.ppu.vblank_stop(&mut self.cpu); // vblank flag OFF
        for (y, scanline) in result.chunks_mut(NES_WIDTH).enumerate() {
            let mut sprites_on_scanline = vec![];
            let sprites_are_8x16 = self.devices.ppu.is_sprite_size_8x16();
            let sprite_tiles_are_in_upper_half = self.devices.ppu.are_sprite_tiles_in_upper_half();
            for sprite_data in self.devices.ppu.oam.chunks_exact(4) {
                let sprite = Sprite::from_oam_data(
                    sprites_are_8x16,
                    sprite_tiles_are_in_upper_half,
                    sprite_data,
                );
                if sprite.is_visible_on_scanline(sprites_are_8x16, y) {
                    if sprites_on_scanline.len() < MAX_SPRITES_PER_SCANLINE {
                        sprites_on_scanline.push(sprite);
                    }
                }
            }
            for (x, pixel) in scanline.iter_mut().enumerate() {
                let (bg_color, bg_palette) = (0, 0); // TODO
                let (sprite_color, sprite_palette, sprite_is_behind_background) =
                    sprites_on_scanline
                        .iter()
                        .filter_map(|s| {
                            s.get_pixel_for_xy(&self.devices.cartridge, sprites_are_8x16, x, y)
                        })
                        .next()
                        .unwrap_or((0, 0, false));
                let background_is_blocking_sprite = bg_color != 0 && sprite_is_behind_background;
                let (color, palette);
                if sprite_color != 0 && !background_is_blocking_sprite {
                    (color, palette) = (sprite_color, sprite_palette);
                } else {
                    (color, palette) = (bg_color, bg_palette);
                }
                *pixel = match color {
                    0 => (x as u32) * 69 + (y as u32) * 420,
                    1 => 0xFF0000,
                    2 => 0x00CC00,
                    3 => 0x0033FF,
                    _ => unreachable!(),
                };
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
    pub fn get_devices(&self) -> &Devices {
        return &self.devices;
    }
}
