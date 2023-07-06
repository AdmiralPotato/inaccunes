use super::Memory;

use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::BitAnd,
};

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
impl Debug for Cpu {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        write!(
            fmt,
            "PC:{pc:04X} A:{a:02X} X:{x:02X} Y:{y:02X} S:01{s:02X} P:{n}{v}1{b}{d}{i}{z}{c}",
            pc = self.pc,
            a = self.a,
            x = self.x,
            y = self.y,
            s = self.s,
            n = if is_bit_set(self.p, STATUS_N) {
                "N"
            } else {
                "n"
            },
            v = if is_bit_set(self.p, STATUS_V) {
                "V"
            } else {
                "v"
            },
            b = if is_bit_set(self.p, STATUS_B) {
                "B"
            } else {
                "b"
            },
            d = if is_bit_set(self.p, STATUS_D) {
                "D"
            } else {
                "d"
            },
            i = if is_bit_set(self.p, STATUS_I) {
                "I"
            } else {
                "i"
            },
            z = if is_bit_set(self.p, STATUS_Z) {
                "Z"
            } else {
                "z"
            },
            c = if is_bit_set(self.p, STATUS_C) {
                "C"
            } else {
                "c"
            },
        )
    }
}

// Bits of the P register.
/// **C**arry flag: whether the last addition carried past 8 bits
#[allow(unused)]
pub const STATUS_C: u8 = 0b_0000_0001;
/// **Z**ero flag: whether the last operation resulted in 0x00
#[allow(unused)]
pub const STATUS_Z: u8 = 0b_0000_0010;
/// **I**nterrupt mask flag: whether interrupts are *disabled* (masked)
#[allow(unused)]
pub const STATUS_I: u8 = 0b_0000_0100;
/// **D**ecimal flag: whether we do addition in a way that makes Solra cry
#[allow(unused)]
pub const STATUS_D: u8 = 0b_0000_1000;
/// **B**reak flag: whether the interrupt was actually a break
#[allow(unused)]
pub const STATUS_B: u8 = 0b_0001_0000;
/// **1** flag: literally hardwired to a one
#[allow(unused)]
pub const STATUS_1: u8 = 0b_0010_0000;
/// o**V**erflow flag: whether the last integer operation did a signed overflow
/// 🤔
#[allow(unused)]
pub const STATUS_V: u8 = 0b_0100_0000;
/// **N**egative flag: whether the last operation resulted in a negative value
#[allow(unused)]
pub const STATUS_N: u8 = 0b_1000_0000;

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

    pub fn reset<M: Memory>(&mut self, memory: &mut M) {
        let a = memory.read_byte(RESET_VECTOR);
        let b = memory.read_byte(RESET_VECTOR + 1);
        self.pc = u16::from_le_bytes([a, b]);
    }

    fn read_pc_and_post_inc<M: Memory>(&mut self, memory: &mut M) -> u8 {
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

    fn decrement<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory).wrapping_sub(1);
        am.put_value(self, memory, value);
        self.assign_status_nz_for_result(value);
    }

    fn increment<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory).wrapping_add(1);
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
    fn and_accumulator<AM: ReadAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        self.a &= am.get_value(self, memory);
        self.assign_status_nz_for_result(self.a);
    }
    fn xor_accumulator<AM: ReadAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        self.a ^= am.get_value(self, memory);
        self.assign_status_nz_for_result(self.a);
    }
    fn bit_test<AM: ReadAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory);
        self.p = assign_bit(self.p, STATUS_Z, value == self.a);
        self.p = (self.p & 0x3F) | (value & 0xC0);
    }
    fn perform_alu_operation<R: WriteAddressingMode<M>, AM: ReadAddressingMode<M>, M: Memory>(
        &mut self,
        memory: &mut M,
        use_carry: bool,
        discard_result: bool,
        subtraction: bool,
    ) {
        let am = AM::new(self, memory);
        let r = R::new(self, memory);
        let thing1 = r.get_value(self, memory);
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
            r.put_value(self, memory, result);
        }
    }
    fn arithmetic_shift_left<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory);
        let carry_out = is_bit_set(value, 0x80);
        let value = value << 1;
        self.assign_status_nz_for_result(value);
        am.put_value(self, memory, value);
        self.p = assign_bit(self.p, STATUS_C, carry_out);
    }
    fn logical_shift_right<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory);
        let carry_out = is_bit_set(value, 0x01);
        let value = value >> 1;
        self.assign_status_nz_for_result(value);
        am.put_value(self, memory, value);
        self.p = assign_bit(self.p, STATUS_C, carry_out);
    }
    fn rotate_left<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory);
        let carry_in = is_bit_set(self.p, STATUS_C);
        let carry_out = is_bit_set(value, 0x80);
        let value = value << 1;
        let value = if carry_in { value | 0x01 } else { value };
        self.assign_status_nz_for_result(value);
        am.put_value(self, memory, value);
        self.p = assign_bit(self.p, STATUS_C, carry_out);
    }
    fn rotate_right<AM: WriteAddressingMode<M>, M: Memory>(&mut self, memory: &mut M) {
        let am = AM::new(self, memory);
        let value = am.get_value(self, memory);
        let carry_in = is_bit_set(self.p, STATUS_C);
        let carry_out = is_bit_set(value, 0x01);
        let value = value >> 1;
        let value = if carry_in { value | 0x80 } else { value };
        self.assign_status_nz_for_result(value);
        am.put_value(self, memory, value);
        self.p = assign_bit(self.p, STATUS_C, carry_out);
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

    pub fn set_nmi_signal(&mut self, active: bool) {
        todo!("NMI signal");
    }

    pub fn set_irq_signal(&mut self, active: bool) {
        todo!("IRQ signal");
    }

    pub fn step<M: Memory>(&mut self, memory: &mut M) {
        //eprintln!("PC is {:X}", self.pc);
        let opcode = self.read_pc_and_post_inc(memory);
        //eprintln!("Opcode is {:02X}", opcode);
        match opcode {
            // BRK xx
            // BReaK the computer
            0x00 => {
                log::warn!(
                    "Executed a BRK instruction at {:04X}. \
                    We have probably entered The Weeds!",
                    self.pc.wrapping_sub(1)
                );
                todo!("interrupt handling");
            }
            // ORA (zp,X)
            // OR with Accumulator (zero page X-indexed indirect)
            0x01 => self.or_accumulator::<ZeroPageXIndexedIndirect, _>(memory),
            // ORA zp
            // OR with Accumulator (zero page)
            0x05 => self.or_accumulator::<ZeroPage, _>(memory),
            // ASL zp
            // Arithmetic Shift Left (zero page)
            0x06 => self.arithmetic_shift_left::<ZeroPage, _>(memory),
            // PHP
            // PusH P (status) register (onto the stack)
            0x08 => self.push_byte(memory, self.p),
            // ORA #imm
            // OR with Accumulator (immediate)
            0x09 => self.or_accumulator::<Immediate, _>(memory),
            // ASL A
            // Arithmetic Shift Left (accumulator)
            0x0A => self.arithmetic_shift_left::<RegisterA, _>(memory),
            // ORA abs
            // OR with Accumulator (absolute)
            0x0D => self.or_accumulator::<Absolute, _>(memory),
            // ASL abs
            // Arithmetic Shift Left (absolute)
            0x0E => self.arithmetic_shift_left::<Absolute, _>(memory),
            // BPL off
            // Branch if PLus (N bit is clear)
            0x10 => self.handle_branch_operation(memory, !is_bit_set(self.p, STATUS_N)),
            // ORA (zp),Y
            // OR with Accumulator (zero page indirect Y-indexed)
            0x11 => self.or_accumulator::<ZeroPageIndirectYIndexed, _>(memory),
            // ORA zp,X
            // OR with Accumulator (zero page x-indexed)
            0x15 => self.or_accumulator::<ZeroPageXIndexed, _>(memory),
            // ASL zp,X
            // Arithmetic Shift Left (zero page X-indexed)
            0x16 => self.arithmetic_shift_left::<ZeroPageXIndexed, _>(memory),
            // CLC
            // CLear Carry
            0x18 => self.p = clear_bit(self.p, STATUS_C),
            // ORA abs,Y
            // OR with Accumulator (absolute Y-indexed)
            0x19 => self.or_accumulator::<AbsoluteYIndexed, _>(memory),
            // ORA abs,X
            // OR with Accumulator (absolute X-indexed)
            0x1D => self.or_accumulator::<AbsoluteXIndexed, _>(memory),
            // ASL abs,X
            // Arithmetic Shift Left (absolute X-indexed)
            0x1E => self.arithmetic_shift_left::<AbsoluteXIndexed, _>(memory),
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
            // AND (zp,X)
            // AND (zero page X-indexed indirect)
            0x21 => self.and_accumulator::<ZeroPageXIndexedIndirect, _>(memory),
            // BIT zp
            // BIT test (zero page)
            0x24 => self.bit_test::<ZeroPage, _>(memory),
            // AND zp
            // AND with accumulator (zero page)
            0x25 => self.and_accumulator::<ZeroPage, _>(memory),
            // ROL zp
            // ROtate Left (zero page)
            0x26 => self.rotate_left::<ZeroPage, _>(memory),
            // PLP
            // PuLl P (status) register (from stack)
            0x28 => self.p = self.pop_byte(memory) | STATUS_1 | STATUS_B,
            // AND #imm
            // AND with accumulator (immediate)
            0x29 => self.and_accumulator::<Immediate, _>(memory),
            // ROL A
            // ROtate Left (accumulator)
            0x2A => self.rotate_left::<RegisterA, _>(memory),
            // BIT abs
            // BIT test (absolute)
            0x2C => self.bit_test::<Absolute, _>(memory),
            // AND abs
            // AND with accumulator (absolute)
            0x2D => self.and_accumulator::<Absolute, _>(memory),
            // ROL abs
            // ROtate Left (absolute)
            0x2E => self.rotate_left::<Absolute, _>(memory),
            // BMI off
            // Branch if MInus (N bit is set)
            0x30 => self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_N)),
            // AND (zp),Y
            // AND with accumulator (zero page indirect Y-indexed)
            0x31 => self.and_accumulator::<ZeroPageIndirectYIndexed, _>(memory),
            // AND zp,X
            // AND with accumulator (zero page x-indexed)
            0x35 => self.and_accumulator::<ZeroPageXIndexed, _>(memory),
            // ROL zp,X
            // ROtate Left (zero page X-indexed)
            0x36 => self.rotate_left::<ZeroPageXIndexed, _>(memory),
            // SEC
            // SEt Carry
            0x38 => self.p = set_bit(self.p, STATUS_C),
            // AND abs,Y
            // AND with accumulator (absolute Y-indexed)
            0x39 => self.and_accumulator::<AbsoluteYIndexed, _>(memory),
            // AND abs,X
            // AND with accumulator (absolute X-indexed)
            0x3D => self.and_accumulator::<AbsoluteXIndexed, _>(memory),
            // ROL abs,X
            // ROtate Left (absolute X-indexed)
            0x3E => self.rotate_left::<AbsoluteXIndexed, _>(memory),
            // RTI
            // ReTurn from Interrupt
            //0x40 => todo!(),
            // EOR (zp,X)
            // Exclusive OR accumulator (zero page X-indexed indirect)
            0x41 => self.xor_accumulator::<ZeroPageXIndexedIndirect, _>(memory),
            // EOR zp
            // Exclusive OR accumulator (zero page)
            0x45 => self.xor_accumulator::<ZeroPage, _>(memory),
            // LSR zp
            // Logical Shift Right (zero page)
            0x46 => self.logical_shift_right::<ZeroPage, _>(memory),
            // PHA
            // PusH A (onto the stack)
            0x48 => {
                self.push_byte(memory, self.a);
            }
            // EOR #imm
            // Exclusive OR accumulator (immediate)
            0x49 => self.xor_accumulator::<Immediate, _>(memory),
            // LSR A
            // Logical Shift Right (accumulator)
            0x4A => self.logical_shift_right::<RegisterA, _>(memory),
            // JMP abs
            // JuMP
            0x4C => self.pc = Absolute::new(self, memory).get_address(),
            // EOR abs
            // Exclusive OR accumulator (absolute)
            0x4D => self.xor_accumulator::<Absolute, _>(memory),
            // LSR abs
            // Logical Shift Right (absolute)
            0x4E => self.logical_shift_right::<Absolute, _>(memory),
            // BVC off
            // Branch if oVerflow Clear
            0x50 => self.handle_branch_operation(memory, !is_bit_set(self.p, STATUS_V)),
            // EOR (zp),Y
            // Exclusive OR accumulator (zero page indirect Y-indexed)
            0x51 => self.xor_accumulator::<ZeroPageIndirectYIndexed, _>(memory),
            // EOR zp,X
            // Exclusive OR accumulator (zero page x-indexed)
            0x55 => self.xor_accumulator::<ZeroPageXIndexed, _>(memory),
            // LSR zp,X
            // Logical Shift Right (zero page X-indexed)
            0x56 => self.logical_shift_right::<ZeroPageXIndexed, _>(memory),
            // CLI
            // CLear the I bit
            0x58 => self.p = clear_bit(self.p, STATUS_I),
            // EOR abs,Y
            // Exclusive OR accumulator (absolute Y-indexed)
            0x59 => self.xor_accumulator::<AbsoluteYIndexed, _>(memory),
            // EOR abs,X
            // Exclusive OR accumulator (absolute X-indexed)
            0x5D => self.xor_accumulator::<AbsoluteXIndexed, _>(memory),
            // LSR abs,X
            // Logical Shift Right (absolute X-indexed)
            0x5E => self.logical_shift_right::<AbsoluteXIndexed, _>(memory),
            // RTS
            // ReTurn from Subroutine
            0x60 => {
                let pc_low = self.pop_byte(memory);
                let pc_high = self.pop_byte(memory);
                let destination = u16::from_le_bytes([pc_low, pc_high]);
                self.pc = destination + 1;
            }

            // ADC (zp,X)
            // ADd with Carry (zero page X-indexed indirect)
            0x61 => self.perform_alu_operation::<RegisterA, ZeroPageXIndexedIndirect, _>(
                memory, true, false, false,
            ),
            // ADC zp
            // ADd with Carry (zero page)
            0x65 => {
                self.perform_alu_operation::<RegisterA, ZeroPage, _>(memory, true, false, false)
            }
            // ROR zp
            // ROtate Right (zero page)
            0x66 => self.rotate_right::<ZeroPage, _>(memory),
            // PLA
            // PuLl A (from the stack)
            0x68 => {
                self.a = self.pop_byte(memory);
                self.assign_status_nz_for_result(self.a);
            }
            // ADC #imm
            // ADd with Carry (immediate)
            0x69 => {
                self.perform_alu_operation::<RegisterA, Immediate, _>(memory, true, false, false)
            }
            // ROR A
            // ROtate Right (accumulator)
            0x6A => self.rotate_right::<RegisterA, _>(memory),
            // JMP (abs)
            // JuMP (absolute indirect)
            0x6C => {
                let address_of_address = Absolute::new(self, memory).get_address();
                let destination_low = memory.read_byte(address_of_address);
                let destination_high = memory.read_byte(address_of_address.wrapping_add(1));
                self.pc = u16::from_le_bytes([destination_low, destination_high]);
            }
            // ADC abs
            // ADd with Carry (absolute)
            0x6D => {
                self.perform_alu_operation::<RegisterA, Absolute, _>(memory, true, false, false)
            }
            // ROR abs
            // ROtate Right (absolute)
            0x6E => self.rotate_right::<Absolute, _>(memory),
            // BVS off
            // Branch if oVerflow Set
            0x70 => self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_V)),
            // ADC (zp),Y
            // ADd with Carry (zero page indirect Y-indexed)
            0x71 => self.perform_alu_operation::<RegisterA, ZeroPageIndirectYIndexed, _>(
                memory, true, false, false,
            ),
            // ADC zp,X
            // ADd with Carry (zero page x-indexed)
            0x75 => self.perform_alu_operation::<RegisterA, ZeroPageXIndexed, _>(
                memory, true, false, false,
            ),
            // ROR zp,X
            // ROtate Right (zero page X-indexed)
            0x76 => self.rotate_right::<ZeroPageXIndexed, _>(memory),
            // SEI
            // SEt the I bit
            0x78 => self.p = set_bit(self.p, STATUS_I),
            // ADC abs,Y
            // ADd with Carry (absolute Y-indexed)
            0x79 => self.perform_alu_operation::<RegisterA, AbsoluteYIndexed, _>(
                memory, true, false, false,
            ),
            // ADC abs,X
            // ADd with Carry (absolute X-indexed)
            0x7D => self.perform_alu_operation::<RegisterA, AbsoluteXIndexed, _>(
                memory, true, false, false,
            ),
            // ROR abs,X
            // ROtate Right (absolute X-indexed)
            0x7E => self.rotate_right::<AbsoluteXIndexed, _>(memory),
            // STA (zp,X)
            // STore Accumulator (zero page X-indexed indirect)
            0x81 => self.store::<RegisterA, ZeroPageXIndexedIndirect, _>(memory),
            // STY zp
            // STore Y (zero page)
            0x84 => self.store::<RegisterY, ZeroPage, _>(memory),
            // STA zp
            // STore Accumulator (zero page)
            0x85 => self.store::<RegisterA, ZeroPage, _>(memory),
            // STX zp
            // STore X (zero page)
            0x86 => self.store::<RegisterX, ZeroPage, _>(memory),
            // DEC Y or DEY
            // DECrement Y
            0x88 => self.decrement::<RegisterY, _>(memory),
            // TXA
            // Transfer X to Accumulator
            0x8A => self.a = self.assign_status_nz_for_result(self.x),
            // STY abs
            // STore Y (absolute)
            0x8C => self.store::<RegisterY, Absolute, _>(memory),
            // STA abs
            // STore Accumulator (absolute)
            0x8D => self.store::<RegisterA, Absolute, _>(memory),
            // STX abs
            // STore X (absolute)
            0x8E => self.store::<RegisterX, Absolute, _>(memory),
            // BCC off
            // Branch if Carry is Clear (C = 0)
            0x90 => self.handle_branch_operation(memory, !is_bit_set(self.p, STATUS_C)),
            // STA (zp),Y
            // STore Accumulator (zero page indirect Y-indexed)
            0x91 => self.store::<RegisterA, ZeroPageIndirectYIndexed, _>(memory),
            // STY zp,X
            // STore Y (zero page X-indexed)
            0x94 => self.store::<RegisterY, ZeroPageXIndexed, _>(memory),
            // STA zp,X
            // STore Accumulator (zero page x-indexed)
            0x95 => self.store::<RegisterA, ZeroPageXIndexed, _>(memory),
            // STX zp,X
            // STore X (zero page Y-indexed)
            0x96 => self.store::<RegisterX, ZeroPageYIndexed, _>(memory),
            // TYA
            // Transfer Y to Accumulator
            0x98 => self.a = self.assign_status_nz_for_result(self.y),
            // STA abs,Y
            // STore Accumulator (absolute Y-indexed)
            0x99 => self.store::<RegisterA, AbsoluteYIndexed, _>(memory),
            // TXS
            // Transfer X to Stack pointer
            0x9A => self.s = self.x, // DOES NOT set flags!
            // STA abs,X
            // STore Accumulator (absolute X-indexed)
            0x9D => self.store::<RegisterA, AbsoluteXIndexed, _>(memory),
            // LDY #imm
            // Load Y (immediate)
            0xA0 => self.load::<RegisterY, Immediate, _>(memory),
            // LDA (zp,X)
            // LoaD Accumulator (zero page X-indexed indirect)
            0xA1 => self.load::<RegisterA, ZeroPageXIndexedIndirect, _>(memory),
            // LDX #imm
            // Load X (immediate)
            0xA2 => self.load::<RegisterX, Immediate, _>(memory),
            // LDY zp
            // LoaD Y (zero page)
            0xA4 => self.load::<RegisterY, ZeroPage, _>(memory),
            // LDA zp
            // LoaD Accumulator (zero page)
            0xA5 => self.load::<RegisterA, ZeroPage, _>(memory),
            // LDX zp
            // LoaD X (zero page)
            0xA6 => self.load::<RegisterX, ZeroPage, _>(memory),
            // TAY
            // Transfer Accumulator to Y
            0xA8 => self.y = self.assign_status_nz_for_result(self.a),
            // LDA #imm
            // LoaD Accumulator (immediate)
            0xA9 => self.load::<RegisterA, Immediate, _>(memory),
            // TAX
            // Transfer Accumulator to X
            0xAA => self.x = self.assign_status_nz_for_result(self.a),
            // LDY abs
            // LoaD Y (absolute)
            0xAC => self.load::<RegisterY, Absolute, _>(memory),
            // LDA abs
            // LoaD Accumulator (absolute)
            0xAD => self.load::<RegisterA, Absolute, _>(memory),
            // LDX abs
            // LoaD X (absolute)
            0xAE => self.load::<RegisterX, Absolute, _>(memory),
            // BCS off
            // Branch if Carry flag is Set
            0xB0 => self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_C)),
            // LDA (zp),Y
            // LoaD Accumulator (zero page indirect Y-indexed)
            0xB1 => self.load::<RegisterA, ZeroPageIndirectYIndexed, _>(memory),
            // LDY zp,X
            // LoaD Y (zero page X-indexed)
            0xB4 => self.load::<RegisterY, ZeroPageXIndexed, _>(memory),
            // LDA zp,X
            // LoaD Accumulator (zero page x-indexed)
            0xB5 => self.load::<RegisterA, ZeroPageXIndexed, _>(memory),
            // LDX zp,Y
            // LoaD X (zero page Y-indexed)
            0xB6 => self.load::<RegisterX, ZeroPageYIndexed, _>(memory),
            // CLV
            // CLear oVerflow
            0xB8 => self.p = clear_bit(self.p, STATUS_V),
            // LDA abs,Y
            // LoaD Accumulator (absolute Y-indexed)
            0xB9 => self.load::<RegisterA, AbsoluteYIndexed, _>(memory),
            // TSX
            // Transfer Stack pointer to X
            0xBA => self.x = self.assign_status_nz_for_result(self.s),
            // LDY abs,X
            // LoaD Y (absolute X-indexed)
            0xBC => self.load::<RegisterY, AbsoluteXIndexed, _>(memory),
            // LDA abs,X
            // LoaD Accumulator (absolute X-indexed)
            0xBD => self.load::<RegisterA, AbsoluteXIndexed, _>(memory),
            // LDX abs,Y
            // LoaD X (absolute Y-indexed)
            0xBE => self.load::<RegisterY, AbsoluteXIndexed, _>(memory),
            // CPY #imm
            // ComPare Y (immediate)
            0xC0 => {
                self.perform_alu_operation::<RegisterY, Immediate, _>(memory, false, true, true)
            }
            // CMP (zp,X)
            // CoMPare accumulator (zero page X-indexed indirect)
            0xC1 => self.perform_alu_operation::<RegisterA, ZeroPageXIndexedIndirect, _>(
                memory, false, true, true,
            ),
            // CPY zp
            // ComPare Y (zero page)
            0xC4 => self.perform_alu_operation::<RegisterY, ZeroPage, _>(memory, false, true, true),
            // CMP zp
            // CoMPare accumulator (zero page)
            0xC5 => self.perform_alu_operation::<RegisterA, ZeroPage, _>(memory, false, true, true),
            // DEC zp
            // DECrement (zero page)
            0xC6 => self.decrement::<ZeroPage, _>(memory),
            // INY
            // INcrement Y
            0xC8 => self.increment::<RegisterY, _>(memory),
            // CMP #imm
            // CoMPare accumulator (immediate)
            0xC9 => {
                self.perform_alu_operation::<RegisterA, Immediate, _>(memory, false, true, true)
            }
            // DEC X or DEX
            // DECrement X
            0xCA => self.decrement::<RegisterX, _>(memory),
            // CPY abs
            // ComPare Y (absolute)
            0xCC => self.perform_alu_operation::<RegisterY, Absolute, _>(memory, false, true, true),
            // CMP abs
            // CoMPare accumulator (absolute)
            0xCD => self.perform_alu_operation::<RegisterA, Absolute, _>(memory, false, true, true),
            // DEC abs
            // DECrement (absolute)
            0xCE => self.decrement::<Absolute, _>(memory),
            // BNE off
            // Branch if Not Equal (Z is clear)
            0xD0 => self.handle_branch_operation(memory, !is_bit_set(self.p, STATUS_Z)),
            // CMP (zp),Y
            // CoMPare accumulator (zero page indirect Y-indexed)
            0xD1 => self.perform_alu_operation::<RegisterA, ZeroPageIndirectYIndexed, _>(
                memory, false, true, true,
            ),
            // CMP zp,X
            // CoMPare accumulator (zero page x-indexed)
            0xD5 => self
                .perform_alu_operation::<RegisterA, ZeroPageXIndexed, _>(memory, false, true, true),
            // DEC zp,X
            // DECrement (zero page X-indexed)
            0xD6 => self.decrement::<ZeroPageXIndexed, _>(memory),
            // CLD
            // CLear Decimal (phew!)
            0xD8 => self.p = clear_bit(self.p, STATUS_D),
            // CMP abs,Y
            // CoMPare accumulator (absolute Y-indexed)
            0xD9 => self
                .perform_alu_operation::<RegisterA, AbsoluteYIndexed, _>(memory, false, true, true),
            // CMP abs,X
            // CoMPare accumulator (absolute X-indexed)
            0xDD => self
                .perform_alu_operation::<RegisterA, AbsoluteXIndexed, _>(memory, false, true, true),
            // DEC abs,X
            // DECrement (absolute X-indexed)
            0xDE => self.decrement::<AbsoluteXIndexed, _>(memory),
            // CPX #imm
            // ComPare X (immediate)
            0xE0 => {
                self.perform_alu_operation::<RegisterX, Immediate, _>(memory, false, true, true)
            }
            // SBC (zp,X)
            // SuBtract with Carry (zero page X-indexed indirect)
            0xE1 => self.perform_alu_operation::<RegisterA, ZeroPageXIndexedIndirect, _>(
                memory, true, false, true,
            ),
            // CPX zp
            // ComPare X (zero page)
            0xE4 => self.perform_alu_operation::<RegisterX, ZeroPage, _>(memory, false, true, true),
            // SBC zp
            // SuBtract with Carry (zero page)
            0xE5 => self.perform_alu_operation::<RegisterA, ZeroPage, _>(memory, true, false, true),
            // INC zp
            // INCrement (zero page)
            0xE6 => self.increment::<ZeroPage, _>(memory),
            // INX
            // INcrement X
            0xE8 => self.increment::<RegisterX, _>(memory),
            // SBC #imm
            // SuBtract with Carry (immediate)
            0xE9 => {
                self.perform_alu_operation::<RegisterA, Immediate, _>(memory, true, false, true)
            }
            // NOP
            // No OPeration
            0xEA => (),
            // CPX abs
            // ComPare X (absolute)
            0xEC => self.perform_alu_operation::<RegisterX, Absolute, _>(memory, false, true, true),
            // SBC abs
            // SuBtract with Carry (absolute)
            0xED => self.perform_alu_operation::<RegisterA, Absolute, _>(memory, true, false, true),
            // INC abs
            // INCrement (absolute)
            0xEE => self.increment::<Absolute, _>(memory),
            // BEQ offset
            // Branch if EQual
            0xF0 => self.handle_branch_operation(memory, is_bit_set(self.p, STATUS_Z)),
            // SBC (zp),Y
            // SuBtract with Carry (zero page indirect Y-indexed)
            0xF1 => self.perform_alu_operation::<RegisterA, ZeroPageIndirectYIndexed, _>(
                memory, true, false, true,
            ),
            // SBC zp,X
            // SuBtract with Carry (zero page x-indexed)
            0xF5 => self
                .perform_alu_operation::<RegisterA, ZeroPageXIndexed, _>(memory, true, false, true),
            // INC zp,X
            // INCrement (zero page X-indexed)
            0xF6 => self.increment::<ZeroPageXIndexed, _>(memory),
            // SED
            // SEt Decimal (nooooo!)
            0xF8 => self.p = set_bit(self.p, STATUS_D),
            // SBC abs,Y
            // SuBtract with Carry (absolute Y-indexed)
            0xF9 => self
                .perform_alu_operation::<RegisterA, AbsoluteYIndexed, _>(memory, true, false, true),
            // SBC abs,X
            // SuBtract with Carry (absolute X-indexed)
            0xFD => self
                .perform_alu_operation::<RegisterA, AbsoluteXIndexed, _>(memory, true, false, true),
            // INC abs,X
            // INCrement (absolute X-indexed)
            0xFE => self.increment::<AbsoluteXIndexed, _>(memory),
            x => panic!("Unknown opcode: {:02X}", x),
        }
        // self.pc = self.pc.wrapping_add(1);
        // self.pc = self.pc.saturating_add(1);
        // self.pc = match self.pc.checked_add(1) {
        //   Some(x) => x,
        //   None => panic!("something else!"),
        // };
    }
    // Ways to inspect the state of the CPU, for debugging and visualization
    // purposes.
    pub fn get_pc(&self) -> u16 {
        self.pc
    }
    pub fn get_a(&self) -> u8 {
        self.a
    }
    pub fn get_x(&self) -> u8 {
        self.x
    }
    pub fn get_y(&self) -> u8 {
        self.y
    }
    pub fn get_s(&self) -> u8 {
        self.s
    }
    pub fn get_p(&self) -> u8 {
        self.p
    }
    // The real 6502 has this feature. They regret adding it. I don't. I think
    // it's rad!
    pub fn set_overflow(&mut self) {
        self.p |= STATUS_V
    }
    // Real 6502s don't have these capabilities, so we'll feature gate them.
    #[cfg(feature = "override-registers")]
    pub fn set_pc(&mut self, nu: u16) {
        self.pc = nu
    }
    #[cfg(feature = "override-registers")]
    pub fn set_a(&mut self, nu: u8) {
        self.a = nu
    }
    #[cfg(feature = "override-registers")]
    pub fn set_x(&mut self, nu: u8) {
        self.x = nu
    }
    #[cfg(feature = "override-registers")]
    pub fn set_y(&mut self, nu: u8) {
        self.y = nu
    }
    #[cfg(feature = "override-registers")]
    pub fn set_s(&mut self, nu: u8) {
        self.s = nu
    }
    #[cfg(feature = "override-registers")]
    pub fn set_p(&mut self, nu: u8) {
        // Especially dangerous since this lets you clear the 1 bit!
        self.p = nu
    }
}
