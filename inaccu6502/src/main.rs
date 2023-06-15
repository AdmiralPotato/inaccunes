mod memory;
use memory::Memory;
mod cpu;
use cpu::Cpu;

struct Computer {
    memory: Memory,
    cpu: Cpu,
}

impl Computer {
    fn new() -> Computer {
        return Computer {
            memory: Memory::new(),
            cpu: Cpu::new(),
        };
    }

    fn reset(&mut self) {
        self.cpu.reset(&self.memory);
    }

    fn step(&mut self) {
        self.cpu.step(&mut self.memory);
    }
}

fn main() {
    let mut computer = Computer::new();
    computer.reset();
    loop {
        computer.step();
    }
}
