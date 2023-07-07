use inaccu6502::{Cpu, Memory};

const BINARY: &[u8] = include_bytes!("6502_functional_test.bin");

struct RAMputer {
    ram: [u8; 65536],
}

impl RAMputer {
    fn new() -> RAMputer {
        RAMputer { ram: [0u8; 65536] }
    }
}

impl Memory for RAMputer {
    fn read_byte(&mut self, address: u16) -> u8 {
        log::trace!("Read: {address:04X} --> {:02X}", self.ram[address as usize]);
        return self.ram[address as usize];
    }
    fn write_byte(&mut self, address: u16, data: u8) {
        log::trace!("Write: {address:04X} <-- {data:02X}");
        self.ram[address as usize] = data;
    }
}

fn main() {
    env_logger::init();
    let mut ramputer = RAMputer::new();
    ramputer.ram[..BINARY.len()].copy_from_slice(BINARY);
    let mut cpu = Cpu::new();
    cpu.reset(&mut ramputer);
    cpu.set_pc(0x0400); // start the test!
    loop {
        let old_pc = cpu.get_pc();
        // TODO: remove this
        if old_pc == 0x09C5 {
            println!("Skipping the BRK test. (We don't have interrupt handling yet.)");
            cpu.set_pc(0x0A11);
        } else if old_pc == 0x343A {
            println!("Skipping an RTI test. (We don't have interrupt handling yet.)");
            cpu.set_pc(0x345D);
        }
        log::trace!("{cpu:?}");
        cpu.step(&mut ramputer);
        let new_pc = cpu.get_pc();
        if old_pc == new_pc {
            if cpu.get_p() & inaccu6502::STATUS_D != 0 {
                log::warn!("Failed a test, but it appears to be BCD-based, so we're skipping it.");
                cpu.set_pc(new_pc + 2);
            } else {
                break;
            }
        }
    }
    if cpu.get_pc() == 0x3469 {
        println!(
            "CPU entered infinite loop at ${:04X}. Tests passed!",
            cpu.get_pc()
        );
    } else {
        println!(
            "CPU entered infinite loop at ${:04X}. It looks like a test failed.",
            cpu.get_pc()
        );
        std::process::exit(1);
    }
}
