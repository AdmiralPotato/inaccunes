use crate::Cpu;

pub trait Memory {
    fn read_byte(&mut self, cpu: &mut Cpu, address: u16) -> u8;
    fn write_byte(&mut self, cpu: &mut Cpu, address: u16, data: u8);
}
