use super::Memory;

use std::ops::BitAnd;

mod addressing_modes;
use addressing_modes::*;

const STACK_BASE: u16 = 0x0100;
const RESET_VECTOR: u16 = 0xFFFC;
const BYTE_SIGN_BIT: u8 = 0x80;
const BYTE_CARRIED_BIT: u16 = 0b1_0000_0000;

pub struct Cpu {
    /// The accumulator. Where math operations can happen.
    a: u8,
    /// Index register #1. Can be the index of a thing.
    x: u8,
    /// Index register #2.
    y: u8,
    /// The stack pointer.
    s: u8,
    /// The status register.
    p: u8,
    /// The program counter.
    pc: u16,
}

// Bits of the P register.
/// **C**arry flag: whether the last addition carried past 8 bits
#[allow(unused)]
const STATUS_C: u8 = 0b_0000_0001;
/// **Z**ero flag: whether the last operation resulted in 0x00
#[allow(unused)]
const STATUS_Z: u8 = 0b_0000_0010;
/// **I**nterrupt mask flag: whether interrupts are *disabled* (masked)
#[allow(unused)]
const STATUS_I: u8 = 0b_0000_0100;
/// **D**ecimal flag: whether we do addition in a way that makes Solra cry
#[allow(unused)]
const STATUS_D: u8 = 0b_0000_1000;
/// **B**reak flag: whether the interrupt was actually a break
#[allow(unused)]
const STATUS_B: u8 = 0b_0001_0000;
/// **1** flag: literally hardwired to a one
#[allow(unused)]
const STATUS_1: u8 = 0b_0010_0000;
/// o**V**erflow flag: whether the last integer operation did a signed overflow
/// 🤔
#[allow(unused)]
const STATUS_V: u8 = 0b_0100_0000;
/// **N**egative flag: whether the last operation resulted in a negative value
#[allow(unused)]
const STATUS_N: u8 = 0b_1000_0000;

fn clear_bit(input: u8, bit: u8) -> u8 {
    input & !bit
}
fn set_bit(input: u8, bit: u8) -> u8 {
    input | bit
}
fn assign_bit(input: u8, bit: u8, whether: bool) -> u8 {
    if whether {
        set_bit(input, bit)
    } else {
        clear_bit(input, bit)
    }
}
fn is_bit_set<A, B>(input: A, bit: B) -> bool
where
    A: BitAnd<B, Output = B>,
    B: PartialEq + Copy,
{
    input & bit == bit
}

impl Cpu {
    pub fn new() -> Cpu {
        return Cpu {
            a: 255,
            x: 255,
            y: 255,
            s: 255,
            p: 255,
            pc: 255,
        };
    }

    pub fn reset<M: Memory>(&mut self, memory: &M) {
        let a = memory.read_byte(RESET_VECTOR);
        let b = memory.read_byte(RESET_VECTOR + 1);
        self.pc = u16::from_le_bytes([a, b]);
    }

    fn read_pc_and_post_inc<M: Memory>(&mut self, memory: &M) -> u8 {
        let value = memory.read_byte(self.pc);
        self.pc += 1;
        return value;
    }

    fn push_byte<M: Memory>(&mut self, memory: &mut M, byte: u8) {
        // 00xx = zero page
        // 01xx = stack (STACK_BASE)
        // xxxx = some other address
        let destination = (self.s) as u16 + STACK_BASE;
        memory.write_byte(destination, byte);
        self.s = self.s.wrapping_sub(1);
    }

    fn pop_byte<M: Memory>(&mut self, memory: &mut M) -> u8 {
        self.s = self.s.wrapping_add(1);
        let destination = (self.s) as u16 + STACK_BASE;
        let result = memory.read_byte(destination);
        return result;
    }

    fn store_zero<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        am.put_value(self, memory, 0);
    }

    fn dec<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory).wrapping_sub(1);
        am.put_value(self, memory, value);
        self.assign_status_nz_for_result(value);
    }

    fn load<Target: WriteAddressingMode<M>, AM: ReadAddressingMode<M>, M: Memory>(
        &mut self,
        memory: &mut M,
    ) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory);
        Target::new(self, memory).put_value(self, memory, value);
        self.assign_status_nz_for_result(value);
    }
    fn store<Source: ReadAddressingMode<M>, AM: WriteAddressingMode<M>, M: Memory>(
        &mut self,
        memory: &mut M,
    ) {
        let value = Source::new(self, memory).get_value(self, memory);
        let am = AM::new(self, memory);
        am.put_value(self, memory, value);
    }
    fn or_accumulator<AM: ReadAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        self.a |= am.get_value(self, memory);
        self.assign_status_nz_for_result(self.a);
    }
    fn add_accumulator<AM: ReadAddressingMode<M>, M: Memory>(
        &mut self,
        memory: &mut M,
        use_carry: bool,
        discard_result: bool,
        subtraction: bool,
    ) {
        let am = AM::new(self, memory);
        let thing1 = self.a;
        let thing2 = if subtraction {
            // -a = (inverted a) + 1
            // a - b = a + (inverted b) + 1
            am.get_value(self, memory) ^ 0xFF
        } else {
            am.get_value(self, memory)
        };
        let thing3 = if is_bit_set(self.p, STATUS_C) && use_carry {
            1
        } else if !use_carry && subtraction {
            1
        } else {
            0
        };
        let result = thing1 as u16 + thing2 as u16 + thing3;
        //  0  1  0
        // 69 72 C4
        // 23 19 77
        // 8C 8C 3B
        //
        // FF + FF + 1 = 0x1FF
        // 100 + 100 = 200
        // 100 + 100 = -56 ?????
        let result = self.assign_status_cnz_for_result(result);
        // oh jeez
        let overflowed = (thing1 ^ result) & (thing2 ^ result) & 0x80 != 0;
        self.p = assign_bit(self.p, STATUS_V, overflowed);
        if !discard_result {
            self.a = result;
        }
    }

    /// Set the N and Z bits in the status register according to the given
    /// result. (Return that same result that was passed in, for convenience.)
    fn assign_status_nz_for_result(&mut self, result: u8) -> u8 {
        self.p = assign_bit(self.p, STATUS_Z, result == 0);
        self.p = assign_bit(self.p, STATUS_N, is_bit_set(result, BYTE_SIGN_BIT));
        result
    }

    /// Set the N, Z, and C bits in the status register according to the given
    /// result. (Return the same result that was passed in, *but as a byte*.)
    fn assign_status_cnz_for_result(&mut self, result: u16) -> u8 {
        self.p = assign_bit(self.p, STATUS_C, is_bit_set(result, BYTE_CARRIED_BIT));
        self.assign_status_nz_for_result(result as u8)
    }

    fn handle_branch_operation<M: Memory>(&mut self, memory: &mut M, should_branch: bool) {
        // casting it to a signed 8-bit integer first means that, when
        // we go to cast it to a u16 below, Rust will "sign extend" it
        let offset = self.read_pc_and_post_inc(memory) as i8;
        // offset 0 -> address + 0
        // offset 1 -> address + 1
        // offset 127 -> address + 127
        // (it's a signed offset, so we wrap around to negative here)
        // offset 128 -> address - 128
        // offset 129 -> address - 127
        // offset 255 -> address - 1
        let potential_destination = self.pc.wrapping_add(offset as u16);
        if should_branch {
            self.pc = potential_destination;
        }
    }

    pub fn step<M: Memory>(&mut self, memory: &mut M) {
        //eprintln!("PC is {:X}", self.pc);
        let opcode = self.read_pc_and_post_inc(memory);
        //eprintln!("Opcode is {:02X}", opcode);
        match opcode {
            // BRK xx
            // BReaK the computer
            0x00 => todo!("BRK"),
            // ORA (zp,X)
            // OR with Accumulator (zero page X-indexed indirect)
            0x01 => self.or_accumulator::<ZeroPageXIndexedIndirect, _>(memory),
            // ORA zp
            // OR with Accumulator (zero page)
            0x05 => self.or_accumulator::<ZeroPage, _>(memory),
            // ASL zp
            // Arithmetic Shift Left (zero page)
            0x06 => todo!("ASL zpg"),
            // PHP
            // PusH P (status) register
            0x08 => todo!("PHP"),
            // ORA #imm
            // OR with Accumulator (immediate)
            0x09 => self.or_accumulator::<Immediate, _>(memory),
            // ASL A
            // Arithmetic Shift Left (accumulator)
            0x0A => todo!("ASL A"),
            0x0D => self.or_accumulator::<Absolute, _>(memory),
            // ASL abs
            // Arithmetic Shift Left (absolute)
            0x0E => todo!("ASL abs"),
            0x10 => self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_N)),
            // CLC
            // CLear Carry
            0x18 => self.p = clear_bit(self.p, STATUS_C),
            // JSR
            // Jump to SubRoutine
            0x20 => {
                let destination_low = self.read_pc_and_post_inc(memory);
                let [pc_low, pc_high] = self.pc.to_le_bytes();
                self.push_byte(memory, pc_high);
                self.push_byte(memory, pc_low);
                let destination_high = self.read_pc_and_post_inc(memory);
                let destination = u16::from_le_bytes([destination_low, destination_high]);
                self.pc = destination;
            }
            // BMI off
            // Branch if MInus (N bit is set)
            0x30 => self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_N)),
            // DEC A (or) DEA
            // DEcrement Accumulator
            0x3A => self.dec::<RegisterA, _>(memory),
            // PHA
            // PusH A (onto the stack)
            0x48 => {
                self.push_byte(memory, self.a);
            }
            // JMP abs
            // JuMP
            0x4C => self.pc = Absolute::new(self, memory).get_address(),
            // PHY
            // PusH Y (onto the stack)
            0x5A => {
                self.push_byte(memory, self.y);
            }
            // RTS
            // ReTurn from Subroutine
            0x60 => {
                let pc_low = self.pop_byte(memory);
                let pc_high = self.pop_byte(memory);
                let destination = u16::from_le_bytes([pc_low, pc_high]);
                self.pc = destination + 1;
            }
            // STZ zp
            // STore Zero (zero page)
            0x64 => self.store_zero::<ZeroPage, _>(memory),
            // PLA
            // PuLl A (from the stack)
            0x68 => {
                self.a = self.pop_byte(memory);
                self.assign_status_nz_for_result(self.a);
            }
            // ADC #imm
            // ADd with Carry (immediate)
            0x69 => self.add_accumulator::<Immediate, _>(memory, true, false, false),
            // PLY
            // PuLl Y (from the stack)
            0x7A => {
                self.y = self.pop_byte(memory);
                self.assign_status_nz_for_result(self.y);
            }
            // BRA offset
            // BRanch Always
            0x80 => {
                self.handle_branch_operation(memory, true);
            }
            // STA zp
            // STore A (zero page)
            0x85 => self.store::<RegisterA, ZeroPage, _>(memory),
            // STA abs
            // STore Accumulator (absolute)
            0x8D => self.store::<RegisterA, Absolute, _>(memory),
            // STX abs
            // STore X (absolute)
            0x8E => self.store::<RegisterX, Absolute, _>(memory),
            // BCC off
            // Branch if Carry is Clear (C = 0)
            0x90 => self.handle_branch_operation(memory, !is_bit_set(self.p, STATUS_C)),
            // TXS
            // Transfer X to Stack pointer
            0x9A => {
                self.s = self.x;
            }
            // STZ abs
            // STore Zero (absolute)
            0x9C => self.store_zero::<Absolute, _>(memory),
            // LDY #imm
            // Load Y (immediate)
            0xA0 => self.load::<RegisterY, Immediate, _>(memory),
            // LDX #imm
            // Load X (immediate)
            0xA2 => self.load::<RegisterY, Immediate, _>(memory),
            // LDA zp
            // LoaD Accumulator (zero page)
            0xA5 => self.load::<RegisterA, ZeroPage, _>(memory),
            // LDA #imm
            // LoaD Accumulator (immediate)
            0xA9 => self.load::<RegisterA, Immediate, _>(memory),
            // LDA abs
            // LoaD Accumulator (absolute)
            0xAD => self.load::<RegisterA, Absolute, _>(memory),
            // BCS offset
            // Branch if Carry flag is Set
            0xB0 => {
                self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_C));
            }
            // LDA (zp),Y
            // LoaD Accumulator (zero page, indirect, Y-indexed)
            0xB1 => self.load::<RegisterA, ZeroPageIndirectYIndexed, _>(memory),
            // INY
            // INcrement Y
            0xC8 => {
                let data = self.y.wrapping_add(1);
                self.y = data;
                self.assign_status_nz_for_result(data);
            }
            // DEC zp
            // DECrement (zero page)
            0xC6 => self.dec::<ZeroPage, _>(memory),
            // CMP imm
            // CoMPare (immediate)
            0xC9 => self.add_accumulator::<Immediate, _>(memory, false, true, true),
            // WAI
            // WAit for Interrupt
            0xCB => {
                // ... we don't have interrupts
                // toDON'T!("this")
            }
            // BNE offset
            // Branch if Not Equal
            0xD0 => {
                self.handle_branch_operation(memory, !is_bit_set(self.p, STATUS_Z));
            }
            // INX
            // INcrement X
            0xE8 => {
                let data = self.x.wrapping_add(1);
                self.x = data;
                self.assign_status_nz_for_result(data);
            }
            // BEQ offset
            // Branch if EQual
            0xF0 => {
                self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_Z));
            }
            x => panic!("Unknown opcode: {:02X}", x),
        }
        // self.pc = self.pc.wrapping_add(1);
        // self.pc = self.pc.saturating_add(1);
        // self.pc = match self.pc.checked_add(1) {
        //   Some(x) => x,
        //   None => panic!("something else!"),
        // };
    }
}
