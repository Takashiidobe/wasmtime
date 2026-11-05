//! Register definitions for LR35902.
//!
use crate::Reg;
use regalloc2::{PReg, PRegSet, RegClass};

/// LR35902 8-bit General Purpose Registers
///
/// These are the ACTUAL LR35902 hardware encodings used in machine code.
/// In most LR35902 instructions, 8-bit registers are encoded as 3 bits:
///   - 000 (0) = B
///   - 001 (1) = C
///   - 010 (2) = D
///   - 011 (3) = E
///   - 100 (4) = H
///   - 101 (5) = L
///   - 110 (6) = (HL) - memory indirect, not a register
///   - 111 (7) = A - accumulator
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Gpr8 {
    B = 0b000,
    C = 0b001,
    D = 0b010,
    E = 0b011,
    H = 0b100,
    L = 0b101,
    A = 0b111,
}

impl Gpr8 {
    pub const ALL: [Self; 7] = [
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::H,
        Self::L,
        Self::A,
    ];
}

#[inline]
#[allow(dead_code)]
pub(crate) fn b() -> Reg {
    gpr(Gpr8::B)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn c() -> Reg {
    gpr(Gpr8::C)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn d() -> Reg {
    gpr(Gpr8::D)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn e() -> Reg {
    gpr(Gpr8::E)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn h() -> Reg {
    gpr(Gpr8::H)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn l() -> Reg {
    gpr(Gpr8::L)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn a() -> Reg {
    gpr(Gpr8::A)
}

/// LR35902 16-bit General Purpose Register Pairs
///
/// These are the ACTUAL LR35902 hardware encodings used in machine code.
/// In most LR35902 instructions, 16-bit register pairs are encoded as 2 bits:
///   00 (0) = BC
///   01 (1) = DE
///   10 (2) = HL
///   11 (3) = AF
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Gprp16 {
    BC = 0b00,
    DE = 0b01,
    HL = 0b10,
    AF = 0b11,
}

//=============================================================================
// Register Constructor Helpers

const fn gpr(r: Gpr8) -> Reg {
    Reg::from_real_reg(preg(r))
}

#[inline]
pub const fn preg(r: Gpr8) -> PReg {
    PReg::new(r as usize, RegClass::Int)
}

pub(crate) fn hw_enc(reg: Reg) -> u8 {
    reg.to_real_reg()
        .expect("LR35902 instruction register must be physical after allocation")
        .hw_enc()
}

pub(crate) const fn allocatable_int_regs() -> PRegSet {
    // pair instructions constrain their component bytes separately.
    PRegSet::empty()
        .with(preg(Gpr8::B))
        .with(preg(Gpr8::C))
        .with(preg(Gpr8::D))
        .with(preg(Gpr8::E))
        .with(preg(Gpr8::H))
        .with(preg(Gpr8::L))
        .with(preg(Gpr8::A))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_registers_are_real_and_keep_the_hardware_encoding() {
        for reg in Gpr8::ALL {
            let physical = gpr(reg).to_real_reg().unwrap();
            assert_eq!(physical.hw_enc(), reg as u8);
            assert_eq!(physical.class(), RegClass::Int);
            assert_eq!(hw_enc(gpr(reg)), reg as u8);
        }
    }

    #[test]
    fn allocatable_set_contains_only_lr35902_byte_registers() {
        let regs = allocatable_int_regs();
        assert_eq!(regs.len(), Gpr8::ALL.len() as u32);
        for reg in Gpr8::ALL {
            assert!(regs.contains(preg(reg)));
        }
        assert!(!regs.contains(PReg::new(6, RegClass::Int)));
        assert!(!regs.contains(PReg::new(8, RegClass::Int)));
    }
}
