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
    pub register_ppudata_address: u16,
    pub cram: [u8; 32],
    pub oam: [u8; 256],
    pub nametables: [u8; 4096],
    vblank_status_flag: bool,
    vblank_in_progress: bool,
    pub is_ppu_address_high: bool,
    pub next_scroll_is_x: bool,
}

impl PPU {
    pub fn new() -> PPU {
        PPU {
            register_control: 0,
            register_mask: 0,
            register_oam_address: 0,
            register_scroll_x: 0,
            register_scroll_y: 0,
            register_ppudata_address: 0,
            oam: [0; 256],
            vblank_status_flag: false,
            vblank_in_progress: false,
            is_ppu_address_high: true,
            next_scroll_is_x: true,
            nametables: [0; 4096],
            cram: [0; 32],
        }
    }
    fn perform_bus_read(&mut self, cartridge: &Cartridge, address: u16) -> u8 {
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
    fn perform_bus_write(&mut self, cartridge: &mut Cartridge, address: u16, data: u8) {
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
        self.register_ppudata_address = self.register_ppudata_address.wrapping_add(inc);
    }
    pub fn perform_register_read(&mut self, cartridge: &Cartridge, address: u16) -> u8 {
        let address = address & 0b111;
        match address {
            0 | 1 | 3 | 5 | 6 => {
                warn!("game read write-only PPU register {address:X}");
                return 0;
            }
            2 => {
                // Reading PPUSTATUS sets the following latches to a known state:
                self.next_scroll_is_x = true;
                self.is_ppu_address_high = true;
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
            4 => {
                todo!("read OAMDATA")
            }
            7 => {
                let result = self.perform_bus_read(cartridge, self.register_ppudata_address);
                self.increment_ppudata_address();
                result
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
                self.register_control = data;
                cpu.set_nmi_signal(self.is_nmi_supposed_to_be_active());
            }
            1 => self.register_mask = data,
            2 => warn!("ROM wrote {data:02X} to PPUSTATUS register"),
            3 => self.register_oam_address = data,
            4 => {
                self.oam[self.register_oam_address as usize] = data;
                self.register_oam_address += 1;
            }
            5 => {
                if self.next_scroll_is_x {
                    self.register_scroll_x = data;
                } else {
                    self.register_scroll_y = data;
                }
                self.next_scroll_is_x = !self.next_scroll_is_x;
            }
            6 => {
                if self.is_ppu_address_high {
                    // Write the high byte
                    self.register_ppudata_address =
                        (self.register_ppudata_address & !0xFF00) | ((data as u16) << 8);
                } else {
                    // Write the low byte
                    self.register_ppudata_address =
                        (self.register_ppudata_address & !0x00FF) | (data as u16);
                }
                self.is_ppu_address_high = !self.is_ppu_address_high;
            }
            7 => {
                self.perform_bus_write(cartridge, self.register_ppudata_address, data);
                self.increment_ppudata_address();
            }
            _ => unreachable!(),
        }
    }
    pub fn vblank_start(&mut self, cpu: &mut Cpu) {
        self.vblank_status_flag = true;
        self.vblank_in_progress = true;
        cpu.set_nmi_signal(self.is_nmi_supposed_to_be_active());
    }
    pub fn vblank_stop(&mut self, cpu: &mut Cpu) {
        self.vblank_status_flag = false;
        self.vblank_in_progress = false;
        cpu.set_nmi_signal(self.is_nmi_supposed_to_be_active());
    }
    fn is_nmi_supposed_to_be_active(&self) -> bool {
        self.register_control & 0x80 != 0 && self.vblank_status_flag
    }
}
