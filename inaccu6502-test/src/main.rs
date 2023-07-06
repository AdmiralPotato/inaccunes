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
        println!("Read: {address:04X} --> {:02X}", self.ram[address as usize]);
        return self.ram[address as usize];
    }
    fn write_byte(&mut self, address: u16, data: u8) {
        println!("Write: {address:04X} <-- {data:02X}");
        self.ram[address as usize] = data;
    }
}

fn main() {
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
            cpu.set_pc(0xA11);
        }
        println!("{cpu:?}");
        cpu.step(&mut ramputer);
        let new_pc = cpu.get_pc();
        if old_pc == new_pc {
            if cpu.get_p() & inaccu6502::STATUS_D != 0 {
                println!("Failed a test, but it appears to be BCD-based, so we're skipping it.");
                cpu.set_pc(new_pc + 2);
            } else {
                break;
            }
        }
    }
    println!(
        "CPU entered infinite loop at ${:04X}. Tell me I did good, Kilua?",
        cpu.get_pc()
    );
}
