use log::*;
use std::{fs::File, io::Read};
pub struct Cartridge {
    pub mirroring_type: MirroringType,
    pub prg_data: Vec<u8>,
    pub chr_data: Vec<u8>,
}

const PRG_CHUNK_SIZE: usize = 16 * 1024; // 16 kibibytes per PRG chunk
const CHR_CHUNK_SIZE: usize = 8 * 1024; // 8 kibibytes per CHR chunk

const HEADER_FLAG_MIRRORING: u8 = 0x01;
const HEADER_FLAG_SAVE_RAM: u8 = 0x02;
const HEADER_FLAG_HAS_TRAINER: u8 = 0x04;
const HEADER_FLAG_FOUR_SCREEN_VRAM: u8 = 0x08;

#[derive(Debug)]
pub enum MirroringType {
    Horizontal,
    Vertical,
    FourScreen,
}

impl Cartridge {
    // TODO: make this return a Result of some kind
    pub fn new(path: &str) -> Self {
        info!("Attempting to open path: '{path}'");
        let mut f = File::open(path).expect("failed to open that file");
        let mut header = [0u8; 16];
        f.read_exact(&mut header)
            .expect("failed to read 16-byte header");
        if &header[0..4] != b"NES\x1A" {
            panic!("It's not an iNES file!");
        }
        let prg_size = header[4] as usize * PRG_CHUNK_SIZE;
        let chr_size = header[5] as usize * CHR_CHUNK_SIZE;
        let flags = header[6];
        let mirroring_type = if flags & HEADER_FLAG_FOUR_SCREEN_VRAM != 0 {
            MirroringType::FourScreen
        } else if flags & HEADER_FLAG_MIRRORING != 0 {
            MirroringType::Vertical
        } else {
            MirroringType::Horizontal
        };
        let has_save_ram = flags & HEADER_FLAG_SAVE_RAM != 0;
        if has_save_ram {
            todo!("implement save ram >:(")
        }
        let has_trainer = flags & HEADER_FLAG_HAS_TRAINER != 0;
        if has_trainer {
            panic!("this archaic ROM has a trainer in it, we don't handle that, FLEE!")
        }
        let mapper_type = flags >> 4;
        match mapper_type {
            0 => {
                // NROM, we're okay
            }
            x => {
                panic!("Unknown mapper type: {}", x)
            }
        }
        info!("ROM info: {prg_size} bytes PRG, {chr_size} bytes CHR, mapper type: {mapper_type}, mirroring type: {mirroring_type:?}");
        let mut prg_data = vec![0; prg_size];
        let mut chr_data = vec![0; chr_size];
        f.read_exact(&mut prg_data)
            .expect("failed to read PRG data");
        f.read_exact(&mut chr_data)
            .expect("failed to read CHR data");
        return Cartridge {
            mirroring_type,
            prg_data,
            chr_data,
        };
    }

    pub fn perform_chr_read(&self, address: u16) -> u8 {
        self.chr_data[(address as usize) % self.chr_data.len()]
    }

    pub(crate) fn perform_chr_write(&mut self, address: u16, data: u8) {
        if false {
            let length = self.chr_data.len();
            self.chr_data[(address as usize) % length] = data;
        } else {
            warn!("We have CHR ROM, but the game wrote {data:02X} to {address:04X}!");
        }
    }
}
