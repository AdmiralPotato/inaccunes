use std::fmt::{Debug, Formatter, Result as FmtResult};

use super::*;

mod ppu;
use inaccu6502::{Cpu, Memory};
use ppu::*;

const TILE_BYTES: usize = 16;
const MAX_SPRITES_PER_SCANLINE: usize = 8;
const BACKGROUND_X_TILE_COUNT: usize = 32;

const BUTTON_A: u8 = /*     */ 0b0000_0001;
const BUTTON_B: u8 = /*     */ 0b0000_0010;
const BUTTON_SELECT: u8 = /**/ 0b0000_0100;
const BUTTON_START: u8 = /* */ 0b0000_1000;
const BUTTON_UP: u8 = /*    */ 0b0001_0000;
const BUTTON_DOWN: u8 = /*  */ 0b0010_0000;
const BUTTON_LEFT: u8 = /*  */ 0b0100_0000;
const BUTTON_RIGHT: u8 = /* */ 0b1000_0000;

fn get_palette_color(grayscale: bool, emphasis: usize, color_index: usize) -> u32 {
    const PALETTE_2C03: &[u8; 1536] = include_bytes!("2c03.pal");
    let color_index = if grayscale {
        color_index & 0x30
    } else {
        color_index & 0x3F
    };
    let index_within_palette = ((emphasis << 6) | color_index) * 3;
    let color_bytes = &PALETTE_2C03[index_within_palette..index_within_palette + 3];
    u32::from_be_bytes([0, color_bytes[0], color_bytes[1], color_bytes[2]])
}

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
                7 - x_within_sprite
            } else {
                x_within_sprite
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
            let sprite_color =
                cartridge.get_tile(self.tile_address, x_within_sprite, y_within_sprite);
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
    fn get_pixel_for_background(
        &mut self,
        cur_nametable: usize,
        tile_x: usize,
        x_within_tile: usize,
        tile_y: usize,
        y_within_tile: usize,
    ) -> (u8, usize) {
        const NAMETABLE_ADDRESSES: [usize; 4] = [0x2000, 0x2400, 0x2800, 0x2C00];
        let nametable_address = NAMETABLE_ADDRESSES[cur_nametable];
        let address_of_tile_number =
            nametable_address + (tile_y * BACKGROUND_X_TILE_COUNT) + tile_x;
        let tile_number = self
            .devices
            .ppu
            .perform_bus_read(&self.devices.cartridge, address_of_tile_number as u16);
        let tile_base_address = if self.devices.ppu.are_bg_tiles_in_upper_half() {
            0x1000
        } else {
            0x0000
        };
        let tile_address = tile_base_address + tile_number as u16 * TILE_BYTES as u16;
        let color = self
            .devices
            .cartridge
            .get_tile(tile_address, x_within_tile, y_within_tile);
        const NUMBER_OF_METATILES_PER_ROW: usize = 8;
        let metatile_x = tile_x / 2;
        let metatile_y = tile_y / 2;
        let index_within_attribute_table =
            (metatile_x / 2) + (metatile_y / 2) * NUMBER_OF_METATILES_PER_ROW;
        let index_within_attribute_byte = (metatile_x % 2) + (metatile_y % 2) * 2;
        let attribute_table_address = nametable_address + 0x3C0;
        let attribute_byte = self.devices.ppu.perform_bus_read(
            &self.devices.cartridge,
            attribute_table_address as u16 + index_within_attribute_table as u16,
        );
        let attribute = (attribute_byte >> (index_within_attribute_byte * 2)) & 0b11;
        (color, attribute as usize)
    }
    fn get_cursed_pixel_for_background(&mut self) -> (u8, usize) {
        let ppu = &mut self.devices.ppu;
        let tile_address_to_read = (ppu.current_render_address & 0x0FFF) | 0x2000;
        let attribute_address_to_read = (ppu.current_render_address & 0x0C00)
            | ((ppu.current_render_address >> 4) & 0x38)
            | ((ppu.current_render_address >> 2) & 0x07)
            | 0x23C0;
        let tile_number = ppu.perform_bus_read(&self.devices.cartridge, tile_address_to_read);
        let tile_base_address = if ppu.are_bg_tiles_in_upper_half() {
            0x1000
        } else {
            0x0000
        };
        let tile_address = tile_base_address + tile_number as u16 * TILE_BYTES as u16;
        let color = self.devices.cartridge.get_tile(
            tile_address,
            ppu.fine_scroll_x as usize,
            (ppu.current_render_address >> 12) as usize,
        );
        let attribute_byte =
            ppu.perform_bus_read(&self.devices.cartridge, attribute_address_to_read as u16);
        let index_within_attribute_byte =
            ((ppu.current_render_address >> 1) & 1) | ((ppu.current_render_address >> 5) & 2);
        let attribute = (attribute_byte >> (index_within_attribute_byte * 2)) & 0b11;
        // scroll!
        ppu.fine_scroll_x += 1;
        if ppu.fine_scroll_x >= 8 {
            ppu.fine_scroll_x = 0;
            // we reached the end of the tile, so go to the next tile
            if ppu.current_render_address & 0b11111 == 0b11111 {
                // if we were at the right edge of the nametable, go to the next
                // nametable
                ppu.current_render_address &= 0b1111111_11100000;
                ppu.current_render_address ^= 0b0000100_00000000;
            } else {
                // we were not at the right edge of the nametable, go to the
                // next tile
                ppu.current_render_address += 1;
            }
        }
        (color, attribute as usize)
    }
    pub fn render(&mut self) -> [u32; NES_PIXEL_COUNT] {
        const CPU_STEPS_PER_SCANLINE: usize = 113;
        const CPU_STEPS_PER_VBLANK: usize = 2273;
        let mut result = [0x0; NES_PIXEL_COUNT];
        // Pretend to be in V-blank.
        // vblank flag ON
        self.devices.ppu.vblank_start(&mut self.cpu);
        for _ in 0..CPU_STEPS_PER_VBLANK {
            self.cpu.step(&mut self.devices);
        }
        // vblank flag OFF
        self.devices.ppu.vblank_stop(&mut self.cpu);
        // BEGIN CURSE!
        self.devices.ppu.current_render_address &= 0b0000100_00011111;
        self.devices.ppu.current_render_address |=
            self.devices.ppu.canon_render_address & 0b1111011_11100000;
        // END CURSE!
        //let mut cur_y_scroll = self.devices.ppu.register_scroll_y as usize;
        for (y, scanline) in result.chunks_mut(NES_WIDTH).enumerate() {
            let mut sprites_on_scanline = vec![];
            let sprites_are_8x16 = self.devices.ppu.is_sprite_size_8x16();
            let sprite_tiles_are_in_upper_half = self.devices.ppu.are_sprite_tiles_in_upper_half();
            for (sprite_index, sprite_data) in self.devices.ppu.oam.chunks_exact(4).enumerate() {
                let sprite = Sprite::from_oam_data(
                    sprites_are_8x16,
                    sprite_tiles_are_in_upper_half,
                    sprite_data,
                );
                if sprite.is_visible_on_scanline(sprites_are_8x16, y) {
                    if sprites_on_scanline.len() < MAX_SPRITES_PER_SCANLINE {
                        sprites_on_scanline.push((sprite_index, sprite));
                    }
                }
            }
            //let mut cur_x_scroll = self.devices.ppu.register_scroll_x as usize;
            //let mut cur_nametable = self.devices.ppu.which_nametable_is_upper_left();
            for (x, pixel) in scanline.iter_mut().enumerate() {
                /*
                let tile_x = cur_x_scroll / 8;
                let x_within_tile = cur_x_scroll % 8;
                let tile_y = cur_y_scroll / 8;
                let y_within_tile = cur_y_scroll % 8;
                let (bg_color, bg_palette) = self.get_pixel_for_background(
                    cur_nametable as usize,
                    tile_x,
                    x_within_tile,
                    tile_y,
                    y_within_tile,
                );
                */
                let (bg_color, bg_palette) = self.get_cursed_pixel_for_background();
                let (sprite_index, (sprite_color, sprite_palette, sprite_is_behind_background)) =
                    sprites_on_scanline
                        .iter()
                        .filter_map(|(index, sprite)| {
                            sprite
                                .get_pixel_for_xy(&self.devices.cartridge, sprites_are_8x16, x, y)
                                .map(|x| (*index, x))
                        })
                        .next()
                        .unwrap_or((69, (0, 0, false)));
                let background_is_blocking_sprite = bg_color != 0 && sprite_is_behind_background;
                let (color, palette);
                if sprite_color != 0 && !background_is_blocking_sprite {
                    (color, palette) = (sprite_color, sprite_palette);
                } else {
                    (color, palette) = (bg_color, bg_palette);
                }
                let color_index = if color == 0 {
                    self.devices.ppu.cram[0] // the "universal background color"
                } else {
                    self.devices.ppu.cram[palette * 4 + color as usize]
                };
                if sprite_index == 0 && bg_color != 0 && sprite_color != 0 {
                    self.devices.ppu.turn_on_sprite_0_hit();
                }
                *pixel = get_palette_color(
                    self.devices.ppu.is_grayscale(),
                    self.devices.ppu.get_emphasis(),
                    color_index as usize,
                );
                // 00000000 XXXXXXXX
                // 00110000 XXXXXXXX
                // 22222222 XXXXXXXX
                //
                // YYYYYYYY ZZZZZZZZ
                // YYYYYYYY ZZZZZZZZ
                // YYYYYYYY ZZZZZZZZ
                /*
                cur_x_scroll += 1;
                if cur_x_scroll >= 256 {
                    cur_x_scroll -= 256;
                    cur_nametable ^= 1;
                }
                */
            }
            for _ in 0..CPU_STEPS_PER_SCANLINE {
                self.cpu.step(&mut self.devices);
            }
            /*
            cur_y_scroll += 1;
            if cur_y_scroll >= 240 {
                cur_y_scroll -= 240;
                self.devices.ppu.flip_which_nametable_is_upper_left_by_y();
            }
            */
            // BEGIN CURSE!
            let ppu = &mut self.devices.ppu;
            // the part of the curse that is about the Y scroll
            ppu.current_render_address += 0b0010000_00000000;
            if ppu.current_render_address >= 0x8000 {
                ppu.current_render_address &= 0b1111111_1111111;
                // If the coarse Y scroll is exactly equal to 29...
                if ppu.current_render_address & (0b11111 << 5) == (29 << 5) {
                    // set it to 0
                    ppu.current_render_address &= !(0b11111 << 5);
                    // and flip to a different nametable
                    ppu.current_render_address ^= 0b10 << 10;
                }
                // Otherwise...
                else {
                    // increment the coarse Y scroll by 1
                    ppu.current_render_address += 0b00001 << 5;
                    // BUG: the thing that happens if you set scroll Y to an
                    // illegal value isn't emulated, DON'T DO THAT ANYWAY
                }
            }
            // the part of the curse that is about the X scroll
            self.devices.ppu.current_render_address &= 0b1111011_11100000;
            self.devices.ppu.current_render_address |=
                self.devices.ppu.canon_render_address & 0b0000100_00011111;
            // END CURSE!
        }
        // we have to do this again at the end of the frame
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
