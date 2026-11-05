//! This module defines LR35902-specific machine instruction types.

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir::{ExternalName, TrapCode, Type, types};
use crate::isa::FunctionAlignment;
use crate::isa::lr35902::settings as lr35902_settings;
use crate::machinst::*;
use crate::{CodegenError, CodegenResult, settings};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::slice;
use smallvec::{SmallVec, smallvec};
use std::fmt::{self, Write};
use std::string::{String, ToString};

mod emit;
mod emit_state;
pub mod regs;

pub use emit_state::EmitState;
use regs::*;

//=============================================================================
// Instructions (top level): definition

// `Inst` is defined inside ISLE as `MInst`. We publicly re-export it here.
pub use super::lower::isle::generated_code::MInst as Inst;

#[test]
#[cfg(target_pointer_width = "64")]
fn inst_size_test() {
    // This test will help with unintentionally growing the size
    // of the Inst enum.
    // TODO: Adjust this size once we define the actual instruction variants
    // assert_eq!(XX, std::mem::size_of::<Inst>());
}

impl Inst {
    /// Check if the instruction can be emitted given the current target
    /// architecture.
    fn is_available(&self, _emit_info: &EmitInfo) -> bool {
        // For now, all instructions are available on LR35902
        true
    }
}

//=============================================================================
// Instruction constructors

impl Inst {
    /// Create a NOP instruction
    pub(crate) fn nop() -> Self {
        Self::Nop
    }

    /// Create a load instruction
    pub(crate) fn load(_ty: Type, _from_addr: impl Into<Amode>, _to_reg: Writable<Reg>) -> Self {
        todo!("Implement load instruction constructor")
    }

    /// Create a store instruction
    pub(crate) fn store(_ty: Type, _from_reg: Reg, _to_addr: impl Into<Amode>) -> Self {
        todo!("Implement store instruction constructor")
    }

    /// Create an immediate load instruction
    pub fn imm(imm_val: u8, dst: Writable<Reg>) -> Inst {
        Inst::LoadImm8 { dst, imm: imm_val }
    }

    /// Create a call to a known function
    pub(crate) fn call_known(info: Box<CallInfo<ExternalName>>) -> Inst {
        Inst::CallKnown { info }
    }

    /// Create an unconditional jump
    pub(crate) fn jmp_known(dst: MachLabel) -> Inst {
        Inst::JumpKnown { dst }
    }

    /// Create a conditional trap
    pub(crate) fn trap_if(_trap_code: TrapCode) -> Inst {
        todo!("Implement trap_if constructor")
    }
}

//=============================================================================
// Instructions: printing

impl PrettyPrint for Inst {
    fn pretty_print(&self, _size: u8) -> String {
        let a = regs::Gpr8::A as u8;
        match self {
            Inst::Nop => "nop".into(),
            Inst::Halt => "halt".into(),
            Inst::Move8 { dst, src } => format!("ld {}, {}", asm_reg(dst.to_reg()), asm_reg(*src)),
            Inst::Add8 { dst, lhs, rhs } => {
                let lhs_enc = regs::hw_enc(*lhs);
                let rhs_enc = regs::hw_enc(*rhs);
                let dst_reg = dst.to_reg();
                let dst_name = asm_reg(dst_reg);
                let mut lines = Vec::new();
                if lhs_enc == a {
                    lines.push(format!("add a, {}", asm_reg(*rhs)));
                } else if rhs_enc == a {
                    lines.push(format!("add a, {}", asm_reg(*lhs)));
                } else {
                    lines.push(format!("ld a, {}", asm_reg(*lhs)));
                    lines.push(format!("add a, {}", asm_reg(*rhs)));
                }
                if regs::hw_enc(dst_reg) != a {
                    lines.push(format!("ld {dst_name}, a"));
                }
                lines.join("\n")
            }
            Inst::Add16 {
                accumulator: _,
                dst_low,
                dst_high,
                lhs_low,
                lhs_high,
                rhs_low,
                rhs_high,
            } => [
                asm_accumulator_op("add", dst_low.to_reg(), *lhs_low, *rhs_low),
                asm_accumulator_op("adc", dst_high.to_reg(), *lhs_high, *rhs_high),
            ]
            .join("\n"),
            Inst::Sub8 { dst, lhs, rhs } => asm_accumulator_op("sub", dst.to_reg(), *lhs, *rhs),
            Inst::Sub16 {
                accumulator: _,
                dst_low,
                dst_high,
                lhs_low,
                lhs_high,
                rhs_low,
                rhs_high,
            } => [
                asm_accumulator_op("sub", dst_low.to_reg(), *lhs_low, *rhs_low),
                asm_accumulator_op("sbc", dst_high.to_reg(), *lhs_high, *rhs_high),
            ]
            .join("\n"),
            Inst::And8 { dst, lhs, rhs } => asm_accumulator_op("and", dst.to_reg(), *lhs, *rhs),
            Inst::Or8 { dst, lhs, rhs } => asm_accumulator_op("or", dst.to_reg(), *lhs, *rhs),
            Inst::Xor8 { dst, lhs, rhs } => asm_accumulator_op("xor", dst.to_reg(), *lhs, *rhs),
            Inst::LoadImm8 { dst, imm } => {
                format!("ld {}, ${imm:02X}", asm_reg(dst.to_reg()))
            }
            Inst::LoadStack8 { dst, mode } => {
                let mut lines = asm_stack_address(stack_mode_offset(*mode, None));
                lines.push("ld a, [hl]".into());
                if regs::hw_enc(dst.to_reg()) != a {
                    lines.push(format!("ld {}, a", asm_reg(dst.to_reg())));
                }
                lines.join("\n")
            }
            Inst::StoreStack8 { src, mode } => {
                let mut lines = Vec::new();
                if regs::hw_enc(*src) != a {
                    lines.push(format!("ld a, {}", asm_reg(*src)));
                }
                lines.extend(asm_stack_address(stack_mode_offset(*mode, None)));
                lines.push("ld [hl], a".into());
                lines.join("\n")
            }
            Inst::AdjustSP { amount } => format!("add sp, {amount}"),
            Inst::DummyUse { .. } | Inst::Args { .. } | Inst::Rets { .. } => String::new(),
            Inst::CallKnown { info } => format!("call {:?}", info.dest),
            Inst::JumpKnown { dst } => format!("jp {}", dst.to_string()),
            Inst::Ret => "ret".into(),
        }
    }
}

fn asm_reg(reg: Reg) -> String {
    match reg.to_real_reg().map(|reg| reg.hw_enc()) {
        Some(0) => "b".into(),
        Some(1) => "c".into(),
        Some(2) => "d".into(),
        Some(3) => "e".into(),
        Some(4) => "h".into(),
        Some(5) => "l".into(),
        Some(7) => "a".into(),
        _ => format!("{reg:?}"),
    }
}

fn asm_accumulator_op(op: &str, dst: Reg, lhs: Reg, rhs: Reg) -> String {
    let a = regs::Gpr8::A as u8;
    let mut lines = Vec::new();
    if regs::hw_enc(lhs) != a {
        lines.push(format!("ld a, {}", asm_reg(lhs)));
    }
    if op == "add" || op == "adc" {
        lines.push(format!("{op} a, {}", asm_reg(rhs)));
    } else {
        lines.push(format!("{op} {}", asm_reg(rhs)));
    }
    if regs::hw_enc(dst) != a {
        lines.push(format!("ld {}, a", asm_reg(dst)));
    }
    lines.join("\n")
}

fn asm_sp_offset(offset: i32) -> String {
    if offset < 0 {
        format!("sp - {}", offset.unsigned_abs())
    } else {
        format!("sp + {offset}")
    }
}

fn asm_stack_address(offset: i32) -> Vec<String> {
    let base = offset.clamp(i8::MIN as i32, i8::MAX as i32);
    let mut lines = vec![format!("ld hl, {}", asm_sp_offset(base))];
    if offset > base {
        lines.extend(core::iter::repeat_n(
            "inc hl".into(),
            (offset - base) as usize,
        ));
    } else if offset < base {
        lines.extend(core::iter::repeat_n(
            "dec hl".into(),
            (base - offset) as usize,
        ));
    }
    lines
}

fn stack_mode_offset(mode: StackAMode, frame: Option<&FrameLayout>) -> i32 {
    match mode {
        StackAMode::Slot(offset) => {
            offset as i32 + frame.map_or(0, |frame| frame.outgoing_args_size as i32)
        }
        StackAMode::OutgoingArg(offset) => offset as i32,
        StackAMode::IncomingArg(offset, _) => {
            let frame_size = frame.map_or(0, |frame| {
                frame.fixed_frame_storage_size
                    + frame.outgoing_args_size
                    + frame.clobber_size
                    + frame.setup_area_size
            });
            i32::try_from(offset).unwrap() + frame_size as i32 + 2
        }
    }
}

#[cfg(test)]
mod pretty_print_tests {
    use super::{Inst, PrettyPrint, Reg, Writable, regs};
    use crate::machinst::abi::StackAMode;

    fn writable(reg: Reg) -> Writable<Reg> {
        Writable::from_reg(reg)
    }

    #[test]
    fn prints_lr35902_instructions_as_rgbds() {
        assert_eq!(
            Inst::LoadImm8 {
                dst: writable(regs::b()),
                imm: 0x2A,
            }
            .pretty_print(0),
            "ld b, $2A"
        );
        assert_eq!(
            Inst::Add8 {
                dst: writable(regs::a()),
                lhs: regs::b(),
                rhs: regs::c(),
            }
            .pretty_print(0),
            "ld a, b\nadd a, c"
        );
        assert_eq!(
            Inst::Sub8 {
                dst: writable(regs::d()),
                lhs: regs::b(),
                rhs: regs::c(),
            }
            .pretty_print(0),
            "ld a, b\nsub c\nld d, a"
        );
        assert_eq!(
            Inst::LoadStack8 {
                dst: writable(regs::c()),
                mode: StackAMode::Slot(-2),
            }
            .pretty_print(0),
            "ld hl, sp - 2\nld a, [hl]\nld c, a"
        );
        assert_eq!(
            Inst::StoreStack8 {
                src: regs::c(),
                mode: StackAMode::Slot(3),
            }
            .pretty_print(0),
            "ld a, c\nld hl, sp + 3\nld [hl], a"
        );
    }

    #[test]
    fn expands_stack_offsets_outside_signed_byte_range() {
        let inst = Inst::LoadStack8 {
            dst: writable(regs::a()),
            mode: StackAMode::Slot(130),
        };

        assert!(
            inst.pretty_print(0)
                .starts_with("ld hl, sp + 127\ninc hl\ninc hl\n")
        );
    }
}

//=============================================================================
// Instructions: MachInst trait implementation

impl MachInst for Inst {
    type ABIMachineSpec = LR35902ABIMachineSpec;

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        match self {
            Inst::Nop | Inst::Halt | Inst::Ret | Inst::JumpKnown { .. } | Inst::AdjustSP { .. } => {
            }
            Inst::CallKnown { info } => {
                let CallInfo {
                    uses,
                    defs,
                    clobbers,
                    ..
                } = &mut **info;
                for CallArgPair { vreg, preg } in uses {
                    collector.reg_fixed_use(vreg, *preg);
                }
                for CallRetPair { vreg, location } in defs {
                    match location {
                        RetLocation::Reg(preg, _) => collector.reg_fixed_def(vreg, *preg),
                        RetLocation::Stack(_, _) => collector.any_def(vreg),
                    }
                }
                collector.reg_clobbers(*clobbers);
            }
            Inst::Move8 { dst, src } => {
                collector.reg_use(src);
                collector.reg_def(dst);
            }
            Inst::Add8 { dst, lhs, rhs } => {
                collector.reg_use(lhs);
                collector.reg_use(rhs);
                collector.reg_def(dst);
                collector.reg_clobbers(regs::preg_set(regs::Gpr8::A));
            }
            Inst::Add16 {
                accumulator,
                dst_low,
                dst_high,
                lhs_low,
                lhs_high,
                rhs_low,
                rhs_high,
            }
            | Inst::Sub16 {
                accumulator,
                dst_low,
                dst_high,
                lhs_low,
                lhs_high,
                rhs_low,
                rhs_high,
            } => {
                collector.reg_fixed_early_def(accumulator, regs::a());
                collector.reg_use(lhs_low);
                collector.reg_use(lhs_high);
                collector.reg_use(rhs_low);
                collector.reg_use(rhs_high);
                collector.reg_early_def(dst_low);
                collector.reg_def(dst_high);
            }
            Inst::Sub8 { dst, lhs, rhs }
            | Inst::And8 { dst, lhs, rhs }
            | Inst::Or8 { dst, lhs, rhs }
            | Inst::Xor8 { dst, lhs, rhs } => {
                collector.reg_use(lhs);
                collector.reg_fixed_use(rhs, regs::b());
                collector.reg_def(dst);
                collector.reg_clobbers(regs::preg_set(regs::Gpr8::A));
            }
            Inst::LoadImm8 { dst, .. } => collector.reg_def(dst),
            Inst::LoadStack8 { dst, .. } => {
                collector.reg_def(dst);
                collector.reg_clobbers(regs::stack_access_clobbers());
            }
            Inst::StoreStack8 { src, .. } => {
                collector.reg_use(src);
                collector.reg_clobbers(regs::stack_access_clobbers());
            }
            Inst::DummyUse { reg } => collector.reg_use(reg),
            Inst::Args { args } => {
                for ArgPair { vreg, preg } in args {
                    collector.reg_fixed_def(vreg, *preg);
                }
            }
            Inst::Rets { rets } => {
                for RetPair { vreg, preg } in rets {
                    collector.reg_fixed_use(vreg, *preg);
                }
            }
        }
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            Inst::Move8 { dst, src } => Some((*dst, *src)),
            _ => None,
        }
    }

    fn is_included_in_clobbers(&self) -> bool {
        !matches!(self, Inst::Args { .. })
    }

    fn is_trap(&self) -> bool {
        // TODO: Detect trap instructions
        false
    }

    fn is_args(&self) -> bool {
        matches!(self, Inst::Args { .. })
    }

    fn is_term(&self) -> MachTerminator {
        match self {
            Inst::Rets { .. } => MachTerminator::Ret,
            Inst::JumpKnown { .. } => MachTerminator::Branch,
            _ => MachTerminator::None,
        }
    }

    fn is_low_level_branch(&self) -> bool {
        false
    }

    fn is_mem_access(&self) -> bool {
        matches!(self, Inst::LoadStack8 { .. } | Inst::StoreStack8 { .. })
    }

    fn gen_move(dst_reg: Writable<Reg>, src_reg: Reg, ty: Type) -> Inst {
        debug_assert_eq!(ty, types::I8);
        Inst::Move8 {
            dst: dst_reg,
            src: src_reg,
        }
    }

    fn gen_nop(_preferred_size: usize) -> Inst {
        Inst::nop()
    }

    fn rc_for_type(ty: &Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match *ty {
            types::I8 => Ok((&[RegClass::Int], &[types::I8])),
            types::I16 => Ok((&[RegClass::Int, RegClass::Int], &[types::I8, types::I8])),
            types::I32 => Ok((
                &[RegClass::Int, RegClass::Int, RegClass::Int, RegClass::Int],
                &[types::I8, types::I8, types::I8, types::I8],
            )),
            types::I64 => Ok((
                &[
                    RegClass::Int,
                    RegClass::Int,
                    RegClass::Int,
                    RegClass::Int,
                    RegClass::Int,
                    RegClass::Int,
                    RegClass::Int,
                    RegClass::Int,
                ],
                &[
                    types::I8,
                    types::I8,
                    types::I8,
                    types::I8,
                    types::I8,
                    types::I8,
                    types::I8,
                    types::I8,
                ],
            )),
            // LR35902 doesn't have native floating-point support
            _ => Err(CodegenError::Unsupported(format!(
                "Unsupported type for LR35902: {ty}"
            ))),
        }
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        match rc {
            RegClass::Int => types::I8,
            _ => panic!("Unexpected register class for LR35902: {:?}", rc),
        }
    }

    fn gen_jump(label: MachLabel) -> Inst {
        Inst::jmp_known(label)
    }

    fn gen_imm_u64(value: u64, dst: Writable<Reg>) -> Option<Self> {
        // LR35902 can only load 8-bit immediates directly
        if value <= 0xFF {
            Some(Inst::imm(value as u8, dst))
        } else {
            None
        }
    }

    fn gen_imm_f64(_value: f64, _tmp: Writable<Reg>, _dst: Writable<Reg>) -> SmallVec<[Self; 2]> {
        // LR35902 doesn't support floating-point
        panic!("LR35902 does not support floating-point operations")
    }

    fn gen_dummy_use(reg: Reg) -> Self {
        Inst::DummyUse { reg }
    }

    fn worst_case_size() -> CodeOffset {
        8
    }

    fn call_type(&self) -> CallType {
        match self {
            Inst::CallKnown { .. } => CallType::Regular,
            _ => CallType::None,
        }
    }

    fn gen_nop_units() -> Vec<Vec<u8>> {
        vec![vec![0x00]]
    }

    fn worst_case_island_growth() -> CodeOffset {
        0
    }

    fn is_safepoint(&self) -> bool {
        matches!(self, Inst::CallKnown { .. })
    }

    fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 1,
            preferred: 1,
        }
    }

    type LabelUse = LabelUse;

    const TRAP_OPCODE: &'static [u8] = &[
        // TODO: Define the trap opcode for LR35902
        // For now, we use HALT (0x76)
        0x76,
    ];
}

//=============================================================================
// EmitInfo: constant state used during emission

/// Constant state used during emissions of a sequence of instructions.
pub struct EmitInfo {
    pub(super) flags: settings::Flags,
    pub(super) isa_flags: lr35902_settings::Flags,
}

impl EmitInfo {
    /// Create a constant state for emission of instructions.
    pub fn new(flags: settings::Flags, isa_flags: lr35902_settings::Flags) -> Self {
        Self { flags, isa_flags }
    }
}

//=============================================================================
// MachInstEmit trait implementation

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Inst>, info: &Self::Info, state: &mut Self::State) {
        emit::emit(self, sink, info, state)
    }

    fn pretty_print_inst(&self, state: &mut Self::State) -> String {
        let inst = match self {
            Inst::LoadStack8 { dst, mode } => Inst::LoadStack8 {
                dst: *dst,
                mode: StackAMode::Slot(i64::from(stack_mode_offset(
                    *mode,
                    Some(&state.frame_layout),
                ))),
            },
            Inst::StoreStack8 { src, mode } => Inst::StoreStack8 {
                src: *src,
                mode: StackAMode::Slot(i64::from(stack_mode_offset(
                    *mode,
                    Some(&state.frame_layout),
                ))),
            },
            _ => self.clone(),
        };
        PrettyPrint::pretty_print(&inst, 0)
    }
}

//=============================================================================
// LabelUse: handling of label references

/// A label-use (internal relocation) in generated code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// An 8-bit relative jump offset
    /// Used by JR instructions (2-byte: opcode + offset)
    JumpRel8,

    /// A 16-bit absolute address
    /// Used by JP and CALL instructions (3-byte: opcode + addr_low + addr_high)
    JumpAbs16,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 1;

    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::JumpRel8 => 127,     // +127 bytes
            LabelUse::JumpAbs16 => 0xFFFF, // 64KB address space
        }
    }

    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::JumpRel8 => 128, // -128 bytes
            LabelUse::JumpAbs16 => 0,  // Absolute addressing
        }
    }

    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::JumpRel8 => 1,  // 8-bit offset
            LabelUse::JumpAbs16 => 2, // 16-bit address
        }
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        match self {
            LabelUse::JumpRel8 => {
                // Calculate relative offset from the end of the instruction
                let rel = (label_offset as i64) - (use_offset as i64) - 1;
                debug_assert!(rel >= -128 && rel <= 127, "JR offset out of range");
                buffer[0] = rel as u8;
            }
            LabelUse::JumpAbs16 => {
                // Store 16-bit address in little-endian format
                let addr = label_offset as u16;
                buffer[0] = (addr & 0xFF) as u8; // Low byte
                buffer[1] = ((addr >> 8) & 0xFF) as u8; // High byte
            }
        }
    }

    fn supports_veneer(self) -> bool {
        // LR35902 doesn't need veneers for any label use
        false
    }

    fn veneer_size(self) -> CodeOffset {
        0
    }

    fn worst_case_veneer_size() -> CodeOffset {
        0
    }

    fn generate_veneer(self, _: &mut [u8], _: CodeOffset) -> (CodeOffset, LabelUse) {
        panic!("LR35902 does not support veneers")
    }

    fn from_reloc(_reloc: Reloc, _addend: Addend) -> Option<Self> {
        // TODO: Map relocation types to LabelUse variants
        None
    }
}

// LR35902 ABI Machine Spec is defined in ../abi.rs
pub use crate::isa::lr35902::abi::LR35902ABIMachineSpec;

//=============================================================================
// Addressing mode type

/// Addressing mode for LR35902 memory operations
#[derive(Clone, Debug)]
pub enum Amode {
    /// Register indirect: (HL), (BC), (DE)
    RegIndirect { base: Reg },

    /// Indexed with displacement: (IX+d), (IY+d)
    IndexedDisp { base: Reg, disp: i8 },

    /// Direct/Absolute addressing: (nn)
    Absolute { addr: u16 },
}

impl From<Reg> for Amode {
    fn from(base: Reg) -> Self {
        Amode::RegIndirect { base }
    }
}

#[cfg(test)]
mod register_tests {
    use super::*;

    #[test]
    fn integer_values_are_split_into_allocatable_bytes() {
        let (classes, bytes) = <Inst as MachInst>::rc_for_type(&types::I16).unwrap();
        assert_eq!(classes, &[RegClass::Int, RegClass::Int]);
        assert_eq!(bytes, &[types::I8, types::I8]);

        let (classes, bytes) = <Inst as MachInst>::rc_for_type(&types::I32).unwrap();
        assert_eq!(classes, &[RegClass::Int; 4]);
        assert_eq!(bytes, &[types::I8; 4]);
    }
}
