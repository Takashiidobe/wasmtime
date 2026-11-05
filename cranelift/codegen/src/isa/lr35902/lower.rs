use crate::ir::{Inst as IRInst, Opcode, types};
use crate::isa::lr35902::LR35902Backend;
use crate::isa::lr35902::inst::*;
use crate::machinst::Reg;
use crate::machinst::lower::*;
use crate::machinst::{Lower, MachLabel, ValueRegs};
use alloc::vec::Vec;
use smallvec::smallvec;

pub mod isle;

impl LowerBackend for LR35902Backend {
    type MInst = Inst;

    fn lower(&self, ctx: &mut Lower<Inst>, ir_inst: IRInst) -> Option<InstOutput> {
        match ctx.data(ir_inst).opcode() {
            Opcode::Iconst if ctx.output_ty(ir_inst, 0) == types::I8 => {
                let imm = u8::try_from(ctx.get_constant(ir_inst)?).ok()?;
                let dst = ctx.alloc_tmp(types::I8).only_reg()?;
                ctx.emit(Inst::imm(imm, dst));
                Some(smallvec![ValueRegs::one(dst.to_reg())])
            }
            Opcode::Return => {
                let args = ctx.dfg().inst_args(ir_inst).to_vec();
                let rets = args
                    .into_iter()
                    .map(|value| ctx.put_value_in_regs(value))
                    .collect::<Vec<_>>();
                ctx.gen_return(&rets);
                Some(smallvec![])
            }
            Opcode::Iadd | Opcode::Isub | Opcode::Band | Opcode::Bor | Opcode::Bxor
                if ctx.output_ty(ir_inst, 0) == types::I8 =>
            {
                let lhs = ctx.put_input_in_regs(ir_inst, 0).only_reg()?;
                let rhs = ctx.put_input_in_regs(ir_inst, 1).only_reg()?;
                let dst = ctx.alloc_tmp(types::I8).only_reg()?;
                let inst = match ctx.data(ir_inst).opcode() {
                    Opcode::Iadd => Inst::Add8 { dst, lhs, rhs },
                    Opcode::Isub => Inst::Sub8 { dst, lhs, rhs },
                    Opcode::Band => Inst::And8 { dst, lhs, rhs },
                    Opcode::Bor => Inst::Or8 { dst, lhs, rhs },
                    Opcode::Bxor => Inst::Xor8 { dst, lhs, rhs },
                    _ => unreachable!(),
                };
                ctx.emit(inst);
                Some(smallvec![ValueRegs::one(dst.to_reg())])
            }
            _ => None,
        }
    }

    fn lower_branch(
        &self,
        ctx: &mut Lower<Inst>,
        ir_inst: IRInst,
        targets: &[MachLabel],
    ) -> Option<()> {
        if ctx.data(ir_inst).opcode() != Opcode::Jump || targets.len() != 1 {
            return None;
        }
        ctx.emit(Inst::jmp_known(targets[0]));
        Some(())
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        None
    }
}
