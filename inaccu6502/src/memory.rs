/*

Address bus is 16-bit

Data bus is 8-bit

0000-3FFF: RAM, 16K
4000-7FFF: IO, 16K
8000-FFFF: ROM, 32K

*/

use std::io::{stdin, stdout, Read, Write};

const RAM_SIZE: u16 = 0x4000; // 16KiB
const RAM_START: u16 = 0x0000;

#[allow(unused)]
const IO_SIZE: u16 = 0x4000; // 16KiB
const IO_START: u16 = 0x4000;

const ROM_SIZE: u16 = 0x8000; // 32KiB
const ROM_START: u16 = 0x8000;

const ROM: &[u8; ROM_SIZE as usize] = include_bytes!("example.rom");

pub struct Memory {
    ram: [u8; RAM_SIZE as usize],
}
impl Memory {
    pub fn new() -> Memory {
        return Memory {
            ram: [0; RAM_SIZE as usize],
        };
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        if address >= ROM_START {
            return ROM[(address - ROM_START) as usize];
        } else if address >= IO_START {
            let mut stdin = stdin();
            let mut buf: [u8; 1] = [0; 1];
            stdin
                .read_exact(&mut buf)
                .expect("Couldn't read from stdin");
            buf[0]
        } else {
            debug_assert_eq!(RAM_START, 0);
            self.ram[address as usize]
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        if address >= ROM_START {
            panic!(
                "NO! DO NOT WRITE TO ROM! Attempted ROM write at address {:X}",
                address
            );
        } else if address >= IO_START {
            if data == 0 {
                // "clear any buffered keys"
                // we don't have those, so we have nothing to clear :D
            } else if data >= 0x80 {
                // clear the screen
                // TODO care about this
            } else {
                let mut stdout = stdout();
                stdout.write_all(&[data])
                    .expect("Couldn't write to stdout, so you probably won't even be able to see this message, oh well");
            }
        } else {
            debug_assert_eq!(RAM_START, 0);
            self.ram[address as usize] = data;
        }
    }
}
