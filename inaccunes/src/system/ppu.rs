use super::*;
use log::*;

use crate::cartridge::{Cartridge, MirroringType};

/*

             /--vertical scroll (horizontal mirrors, Kid Icarus)
             vv-horizontal scroll (vertical mirrors, Super Mario Bros.)
$2000 - xxxx 00xx xxxx xxxx - top left
$2400 - xxxx 01xx xxxx xxxx - top right
$2800 - xxxx 10xx xxxx xxxx - bottom left
$2C00 - xxxx 11xx xxxx xxxx - bottom right
_____________//|| |||| ||||
_____________/ || |||| ||||
<-- cartridge  || |||| ||||
______________ || |||| ||||
              \|| |||| ||||
              yyy yyyy yyyy - the nametable RAM (2048 bytes)


*/

pub struct PPU {
    pub register_control: u8,
    pub register_mask: u8,
    pub register_oam_address: u8,
    pub register_scroll_x: u8,
    pub register_scroll_y: u8,
    //pub register_ppudata_address: u16,
    pub cram: [u8; 32],
    pub oam: [u8; 256],
    pub nametables: [u8; 4096],
    vblank_status_flag: bool,
    vblank_in_progress: bool,
    pub cursed_multi_register_flag: bool,
    sprite_0_hit_flag: bool,
    ppudata_latch: u8,
    // reference: https://forums.nesdev.org/viewtopic.php?t=664
    pub current_render_address: u16, // LoopyV
    pub canon_render_address: u16,   // LoopyT
    pub fine_scroll_x: u8,
}

impl PPU {
    pub fn new() -> PPU {
        PPU {
            register_control: 0,
            register_mask: 0,
            register_oam_address: 0,
            register_scroll_x: 0,
            register_scroll_y: 0,
            //register_ppudata_address: 0,
            oam: [0; 256],
            vblank_status_flag: false,
            vblank_in_progress: false,
            cursed_multi_register_flag: true,
            nametables: [0; 4096],
            cram: [0; 32],
            sprite_0_hit_flag: false,
            ppudata_latch: 0,
            current_render_address: 0,
            canon_render_address: 0,
            fine_scroll_x: 0,
        }
    }
    pub fn perform_bus_read(&mut self, cartridge: &Cartridge, address: u16) -> u8 {
        // only 14 bits of address exist on the bus
        let address = address & 0b11_1111_1111_1111;
        if address < 0x2000 {
            cartridge.perform_chr_read(address)
        } else if address > 0x3F00 {
            let cram_address = address & 0x1F;
            self.cram[cram_address as usize]
        } else {
            self.nametables[(address & 0b1111_1111_1111) as usize]
        }
    }
    pub fn perform_bus_write(&mut self, cartridge: &mut Cartridge, address: u16, data: u8) {
        // only 14 bits of address exist on the bus
        let address = address & 0b11_1111_1111_1111;
        if address < 0x2000 {
            cartridge.perform_chr_write(address, data)
        } else if address > 0x3F00 {
            let cram_address = address & 0x1F;
            self.cram[cram_address as usize] = data;
        } else {
            let bit_to_flip = match cartridge.mirroring_type {
                MirroringType::Horizontal => 0b0100_0000_0000,
                MirroringType::Vertical => 0b1000_0000_0000,
                MirroringType::FourScreen => 0b0000_0000_0000,
            };
            let nametable_address = address & 0xFFF;
            self.nametables[nametable_address as usize] = data;
            self.nametables[(nametable_address ^ bit_to_flip) as usize] = data;
        }
    }
    fn increment_ppudata_address(&mut self) {
        let inc = if (self.register_control & 0x4) == 0 {
            1
        } else {
            32
        };
        self.current_render_address = self.current_render_address.wrapping_add(inc);
    }
    pub fn perform_register_read(&mut self, cartridge: &Cartridge, address: u16) -> u8 {
        let address = address & 0b111;
        match address {
            0 | 1 | 3 | 5 | 6 => {
                warn!("game read write-only PPU register {address:X}");
                return 0;
            }
            2 => {
                // Reading PPUSTATUS sets the latch to a known state:
                self.cursed_multi_register_flag = true;
                let mut result = 0;
                // Sprite Overflow flag. The real hardware is buggy as
                // hell. For now, we won't try to implement it.
                if false {
                    result |= 0x20;
                }
                // Sprite 0 Hit flag.
                if self.sprite_0_hit_flag {
                    result |= 0x40;
                }
                // Vertical Blank flag.
                if self.vblank_status_flag {
                    result |= 0x80;
                    self.vblank_status_flag = false;
                }
                result
            }
            4 => {
                todo!("read OAMDATA")
            }
            7 => {
                let real_result = self.perform_bus_read(cartridge, self.current_render_address);
                let output_result = self.ppudata_latch;
                self.ppudata_latch = real_result;
                self.increment_ppudata_address();
                output_result
            }
            _ => unreachable!(),
        }
    }
    pub fn perform_register_write(
        &mut self,
        cpu: &mut Cpu,
        cartridge: &mut Cartridge,
        address: u16,
        data: u8,
    ) {
        let address = address & 0b111;
        match address {
            0 => {
                // BEGIN CURSE!
                let loopy_bits = data & 0b11;
                self.canon_render_address &= 0b1110011_11111111;
                self.canon_render_address |= (loopy_bits as u16) << 10;
                // END CURSE!
                self.register_control = data;
                cpu.set_nmi_signal(self.is_nmi_supposed_to_be_active());
            }
            1 => self.register_mask = data,
            2 => warn!("ROM wrote {data:02X} to PPUSTATUS register"),
            3 => self.register_oam_address = data,
            4 => {
                self.oam[self.register_oam_address as usize] = data;
                self.register_oam_address = self.register_oam_address.wrapping_add(1);
            }
            5 => {
                if self.cursed_multi_register_flag {
                    self.register_scroll_x = data;
                    // BEGIN CURSE!
                    let loopy_data = data >> 3;
                    self.canon_render_address &= 0b1111111_11100000;
                    self.canon_render_address |= (loopy_data as u16) << 0;
                    let loopy_data = data & 0b111;
                    self.fine_scroll_x = loopy_data;
                    // END CURSE!
                } else {
                    self.register_scroll_y = data;
                    // BEGIN CURSE!
                    self.canon_render_address &= 0b0001100_00011111;
                    let loopy_data = data >> 3;
                    self.canon_render_address |= (loopy_data as u16) << 5;
                    let loopy_data = data & 0b111;
                    self.canon_render_address |= (loopy_data as u16) << 12;
                    // END CURSE!
                }
                self.cursed_multi_register_flag = !self.cursed_multi_register_flag;
            }
            6 => {
                if self.cursed_multi_register_flag {
                    // Write the high byte
                    // log::trace!("PPUADDR high write: {data:02X}");
                    //self.register_ppudata_address =
                    //    (self.register_ppudata_address & !0xFF00) | ((data as u16) << 8);
                    // BEGIN CURSE!
                    self.canon_render_address &= 0b1000000_11111111;
                    let loopy_data = data & 0b111111;
                    self.canon_render_address |= (loopy_data as u16) << 8;
                    // END CURSE!
                } else {
                    // Write the low byte
                    // log::trace!("PPUADDR low write:  {data:02X}");
                    //self.register_ppudata_address =
                    //    (self.register_ppudata_address & !0x00FF) | (data as u16);
                    // BEGIN CURSE!
                    self.canon_render_address &= 0b1111111_00000000;
                    let loopy_data = data;
                    self.canon_render_address |= (loopy_data as u16) << 0;
                    // AND!
                    self.current_render_address = self.canon_render_address;
                    // END CURSE!
                }
                self.cursed_multi_register_flag = !self.cursed_multi_register_flag;
            }
            7 => {
                self.perform_bus_write(cartridge, self.current_render_address, data);
                self.increment_ppudata_address();
            }
            _ => unreachable!(),
        }
    }
    pub fn vblank_start(&mut self, cpu: &mut Cpu) {
        self.vblank_status_flag = true;
        self.vblank_in_progress = true;
        cpu.set_nmi_signal(self.is_nmi_supposed_to_be_active());
        self.sprite_0_hit_flag = true;
    }
    pub fn vblank_stop(&mut self, cpu: &mut Cpu) {
        self.vblank_status_flag = false;
        self.vblank_in_progress = false;
        cpu.set_nmi_signal(self.is_nmi_supposed_to_be_active());
        self.sprite_0_hit_flag = false;
    }
    fn is_nmi_supposed_to_be_active(&self) -> bool {
        self.is_nmi_on() && self.vblank_status_flag
    }
    pub fn is_nmi_on(&self) -> bool {
        (self.register_control & 0x80) != 0
    }
    pub fn is_master(&self) -> bool {
        (self.register_control & 0x40) == 0
    }
    pub fn is_sprite_size_8x16(&self) -> bool {
        (self.register_control & 0x20) != 0
    }
    pub fn are_bg_tiles_in_upper_half(&self) -> bool {
        (self.register_control & 0x10) != 0
    }
    pub fn are_sprite_tiles_in_upper_half(&self) -> bool {
        (self.register_control & 0x8) != 0
    }
    pub fn is_vram_incrementing_by_y(&self) -> bool {
        (self.register_control & 0x4) != 0
    }
    pub fn which_nametable_is_upper_left(&self) -> u8 {
        self.register_control & 3
    }
    pub fn flip_which_nametable_is_upper_left_by_y(&mut self) {
        self.register_control ^= 2
    }
    pub fn is_grayscale(&self) -> bool {
        let data = self.register_mask;
        if (data & 0b1) == 0 {
            false
        } else {
            true
        }
    }
    pub fn get_emphasis(&self) -> usize {
        let data = self.register_mask;
        (data >> 5) as usize
    }
    pub fn turn_on_sprite_0_hit(&mut self) {
        self.sprite_0_hit_flag = true;
    }
}
