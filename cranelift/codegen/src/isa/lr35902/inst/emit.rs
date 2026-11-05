//! Emission of LR35902 machine code.

use crate::machinst::{MachBuffer, VCodeConstants};

use super::{EmitInfo, EmitState, Inst};

/// Emit a single instruction to the code buffer.
pub(crate) fn emit(
    inst: &Inst,
    sink: &mut MachBuffer<Inst>,
    info: &EmitInfo,
    state: &mut EmitState,
) {
    match inst {
        Inst::Nop => {
            emit_nop(sink);
        }
        Inst::Halt => {
            emit_halt(sink);
        }
        Inst::Move8 { dst, src } => {
            emit_ld_r_r(
                sink,
                super::regs::hw_enc(dst.to_reg()),
                super::regs::hw_enc(*src),
            );
        }
        Inst::DummyUse { .. } | Inst::Args { .. } | Inst::Rets { .. } => {}
        Inst::Ret => sink.put1(0xC9),
    }
}

/// Emit a NOP instruction (0x00)
/// Encoding: 0000_0000
fn emit_nop(sink: &mut MachBuffer<Inst>) {
    sink.put1(0x00);
}

/// Emit a HALT instruction (0x76)
/// Encoding: 0111_0110
fn emit_halt(sink: &mut MachBuffer<Inst>) {
    sink.put1(0x76);
}

/// Emit an LD r, n instruction (load 8-bit immediate into register)
/// Encoding: 00_rrr_110 n
/// Example: LD A, 42  =>  0x3E 0x2A  (A=111, so 00_111_110 = 0x3E)
#[allow(dead_code)]
fn emit_ld_r_imm8(sink: &mut MachBuffer<Inst>, reg_hw_enc: u8, imm: u8) {
    debug_assert!(reg_hw_enc <= 7, "8-bit register encoding must be 0-7");
    let opcode = 0b00_000_110 | (reg_hw_enc << 3);
    sink.put1(opcode);
    sink.put1(imm);
}

/// Emit an LD rp, nn instruction (load 16-bit immediate into register pair)
/// Encoding: 00_rp_0001 n_lo n_hi
#[allow(dead_code)]
fn emit_ld_rp_imm16(sink: &mut MachBuffer<Inst>, reg_hw_enc: u8, imm: u16) {
    debug_assert!(reg_hw_enc <= 3, "16-bit register encoding must be 0-3");
    let opcode = 0b00_000_001 | (reg_hw_enc << 4);
    sink.put1(opcode);
    sink.put1((imm & 0x00FF) as u8); // Low byte
    sink.put1(((imm & 0xFF00) >> 8) as u8); // High byte
}

/// Emit an LD r, r' instruction (load register to register)
/// Encoding: 01_ddd_sss
/// Example: LD A, B  =>  0x78  (dst=A=111, src=B=000, so 01_111_000 = 0x78)
#[allow(dead_code)]
fn emit_ld_r_r(sink: &mut MachBuffer<Inst>, dst_hw_enc: u8, src_hw_enc: u8) {
    debug_assert!(dst_hw_enc <= 7 && src_hw_enc <= 7);
    // Note: 110 (6) is reserved for (HL) memory indirect
    debug_assert!(
        dst_hw_enc != 6 && src_hw_enc != 6,
        "reg encoding 6 is for (HL)"
    );
    let opcode = 0b01_000_000 | (dst_hw_enc << 3) | src_hw_enc;
    sink.put1(opcode);
}

// Example of how to use register encodings in emit functions:
//
// ```rust
// use super::regs::{hw_enc_8bit, hw_enc_16bit, index_reg_prefix};
// use super::Inst;
//
// fn emit_my_instruction(inst: &Inst, sink: &mut MachBuffer<Inst>) {
//     match inst {
//         Inst::LoadImm8 { dst, imm } => {
//             // Get the hardware encoding for the destination register
//             let dst_enc = hw_enc_8bit(*dst).expect("invalid 8-bit register");
//             emit_ld_r_imm8(sink, dst_enc, *imm);
//         }
//         Inst::Move8 { dst, src } => {
//             let dst_enc = hw_enc_8bit(*dst).expect("invalid dst register");
//             let src_enc = hw_enc_8bit(*src).expect("invalid src register");
//             emit_ld_r_r(sink, dst_enc, src_enc);
//         }
//         Inst::LoadIndexed { dst, index, disp } => {
//             // IX/IY instructions need a prefix byte
//             if let Some(prefix) = index_reg_prefix(*index) {
//                 put_u8(sink, prefix);  // DD or FD prefix
//             }
//             let dst_enc = hw_enc_8bit(*dst).expect("invalid dst register");
//             // Then emit the actual instruction...
//         }
//         _ => todo!(),
//     }
// }
// ```

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_control::ControlPlane;

    #[test]
    fn test_ld_a_imm() {
        // Test: LD A, 42 should encode as 0x3E 0x2A
        // A = 7 (0b111), so opcode = 00_111_110 = 0x3E
        use crate::machinst::MachBuffer;
        let mut sink = MachBuffer::<Inst>::new();
        emit_ld_r_imm8(&mut sink, 7, 0x2A); // A=7, imm=42

        let code = sink.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(code.data(), &[0x3E, 0x2A]);
    }

    #[test]
    fn test_instruction_dispatch_emits_nop() {
        use crate::isa::lr35902::settings as lr35902_settings;
        use crate::machinst::MachInstEmit;
        use crate::settings;

        let flags = settings::Flags::new(settings::builder());
        let isa_flags = lr35902_settings::Flags::new(&flags, &lr35902_settings::builder());
        let info = EmitInfo::new(flags, isa_flags);
        let mut state = EmitState::default();
        let mut sink = MachBuffer::<Inst>::new();
        MachInstEmit::emit(&Inst::Nop, &mut sink, &info, &mut state);
        let code = sink.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(code.data(), &[0x00]);
    }

    #[test]
    fn test_register_move_emission() {
        use crate::isa::lr35902::inst::regs;
        use crate::machinst::{MachInst, Writable};

        let inst = <Inst as MachInst>::gen_move(
            Writable::from_reg(regs::a()),
            regs::b(),
            crate::ir::types::I8,
        );
        let flags = crate::settings::Flags::new(crate::settings::builder());
        let isa_flags = crate::isa::lr35902::settings::Flags::new(
            &flags,
            &crate::isa::lr35902::settings::builder(),
        );
        let info = EmitInfo::new(flags, isa_flags);
        let mut state = EmitState::default();
        let mut sink = MachBuffer::<Inst>::new();
        crate::machinst::MachInstEmit::emit(&inst, &mut sink, &info, &mut state);
        let code = sink.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(code.data(), &[0x78]);
    }

    #[test]
    fn test_halt() {
        let mut sink = MachBuffer::<Inst>::new();
        emit_halt(&mut sink);
        let code = sink.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(code.data(), &[0x76]);
    }

    #[test]
    fn test_ld_a_b() {
        // Test: LD A, B should encode as 0x78
        // dst=A=7 (0b111), src=B=0 (0b000)
        // opcode = 01_111_000 = 0x78
        use crate::machinst::MachBuffer;
        let mut sink = MachBuffer::<Inst>::new();
        emit_ld_r_r(&mut sink, 7, 0); // dst=A=7, src=B=0

        let code = sink.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(code.data(), &[0x78]);
    }

    #[test]
    fn emits_return_opcode() {
        let flags = crate::settings::Flags::new(crate::settings::builder());
        let isa_flags = crate::isa::lr35902::settings::Flags::new(
            &flags,
            &crate::isa::lr35902::settings::builder(),
        );
        let info = EmitInfo::new(flags, isa_flags);
        let mut state = EmitState::default();
        let mut sink = MachBuffer::<Inst>::new();
        crate::machinst::MachInstEmit::emit(&Inst::Ret, &mut sink, &info, &mut state);
        let code = sink.finish(&VCodeConstants::default(), &mut ControlPlane::default());
        assert_eq!(code.data(), &[0xC9]);
    }
}
