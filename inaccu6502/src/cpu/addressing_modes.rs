use super::{Cpu, Memory};

/// An addressing mode that we can get a value from.
pub trait ReadAddressingMode {
    fn new(cpu: &mut Cpu, memory: &mut Memory) -> Self;
    fn get_value(&self, cpu: &Cpu, memory: &mut Memory) -> u8;
}
/// An addressing mode that we can (also) put a value into.
pub trait WriteAddressingMode: ReadAddressingMode {
    fn put_value(&self, cpu: &mut Cpu, memory: &mut Memory, value: u8);
}
pub trait AddressibleAddressingMode: ReadAddressingMode {
    fn get_address(&self) -> u16;
}

pub struct Immediate(u8);
impl ReadAddressingMode for Immediate {
    fn new(cpu: &mut Cpu, memory: &mut Memory) -> Self {
        return Self(cpu.read_pc_and_post_inc(memory));
    }
    fn get_value(&self, _cpu: &Cpu, _memory: &mut Memory) -> u8 {
        let Self(value) = self;
        return *value;
    }
}

macro_rules! addressible_mode {
    (
        name: $name:ident,
        cpu_var_name: $cpu:ident,
        memory_var_name: $memory:ident,
        new_function_body: $code:block
    ) => {
        /*
        There are two kinds of structs-with-fields that you can have. The kind
        that is delimited with braces has fields with names. The kind that is
        delimited with parentheses has fields with positions instead.
        */
        pub struct $name(u16);
        //pub struct ZeroPage { address: u16 }
        impl ReadAddressingMode for $name {
            fn new($cpu: &mut Cpu, $memory: &mut Memory) -> Self {
                $code
            }
            fn get_value(&self, _cpu: &Cpu, memory: &mut Memory) -> u8 {
                // destructuring assignment of 0th positional value into `address`
                let Self(source) = self;
                memory.read_byte(*source)
            }
        }
        impl WriteAddressingMode for $name {
            fn put_value(&self, _cpu: &mut Cpu, memory: &mut Memory, value: u8) {
                let Self(destination) = self;
                memory.write_byte(*destination, value);
            }
        }
        impl AddressibleAddressingMode for $name {
            fn get_address(&self) -> u16 {
                // destructuring assignment of 0th positional value into `address`
                let Self(source) = self;
                return *source;
            }
        }
    };
}

addressible_mode!(
    name: ZeroPage,
    cpu_var_name: cpu,
    memory_var_name: memory,
    new_function_body: {
        let address = cpu.read_pc_and_post_inc(memory);
        Self(address as u16)
    }
);
addressible_mode!(
    name: ZeroPageIndirectYIndexed,
    cpu_var_name: cpu,
    memory_var_name: memory,
    new_function_body: {
        let address_of_address = cpu.read_pc_and_post_inc(memory);
        let base_low = memory.read_byte(address_of_address as u16);
        let base_high = memory.read_byte(address_of_address as u16 + 1);
        let base = u16::from_le_bytes([base_low, base_high]);
        return Self(base.wrapping_add(cpu.y as u16));
    }
);
addressible_mode!(
    name: Absolute,
    cpu_var_name: cpu,
    memory_var_name: memory,
    new_function_body: {
        let a = cpu.read_pc_and_post_inc(memory);
        let b = cpu.read_pc_and_post_inc(memory);
        let address = u16::from_le_bytes([a, b]);
        return Self(address);
    }
);

macro_rules! register_mode {
    ($name:ident, $field:ident) => {
        pub struct $name;
        impl ReadAddressingMode for $name {
            fn new(_cpu: &mut Cpu, _memory: &mut Memory) -> Self {
                return Self;
            }
            fn get_value(&self, cpu: &Cpu, _memory: &mut Memory) -> u8 {
                cpu.$field
            }
        }
        impl WriteAddressingMode for $name {
            fn put_value(&self, cpu: &mut Cpu, _memory: &mut Memory, value: u8) {
                cpu.$field = value;
            }
        }
    };
}

register_mode!(RegisterA, a);
register_mode!(RegisterX, x);
register_mode!(RegisterY, y);
