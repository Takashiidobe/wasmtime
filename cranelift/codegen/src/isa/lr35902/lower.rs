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
                let imm = ctx.get_constant(ir_inst)? as u8;
                let dst = ctx.alloc_tmp(types::I8).only_reg()?;
                ctx.emit(Inst::imm(imm, dst));
                Some(smallvec![ValueRegs::one(dst.to_reg())])
            }
            Opcode::Iconst if ctx.output_ty(ir_inst, 0) == types::I16 => {
                let imm = ctx.get_constant(ir_inst)? as u16;
                let dst = ctx.alloc_tmp(types::I16);
                let dst_low = dst.regs()[0];
                let dst_high = dst.regs()[1];
                ctx.emit(Inst::imm(imm as u8, dst_low));
                ctx.emit(Inst::imm((imm >> 8) as u8, dst_high));
                Some(smallvec![ValueRegs::two(
                    dst_low.to_reg(),
                    dst_high.to_reg(),
                )])
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
            Opcode::Iadd | Opcode::Isub if ctx.output_ty(ir_inst, 0) == types::I16 => {
                let lhs = ctx.put_input_in_regs(ir_inst, 0);
                let rhs = ctx.put_input_in_regs(ir_inst, 1);
                let dst = ctx.alloc_tmp(types::I16);
                let [lhs_low, lhs_high] = lhs.regs() else {
                    return None;
                };
                let [rhs_low, rhs_high] = rhs.regs() else {
                    return None;
                };
                let [dst_low, dst_high] = dst.regs() else {
                    return None;
                };
                let accumulator = ctx.alloc_tmp(types::I8).only_reg()?;
                let inst = if ctx.data(ir_inst).opcode() == Opcode::Iadd {
                    Inst::Add16 {
                        accumulator,
                        dst_low: *dst_low,
                        dst_high: *dst_high,
                        lhs_low: *lhs_low,
                        lhs_high: *lhs_high,
                        rhs_low: *rhs_low,
                        rhs_high: *rhs_high,
                    }
                } else {
                    Inst::Sub16 {
                        accumulator,
                        dst_low: *dst_low,
                        dst_high: *dst_high,
                        lhs_low: *lhs_low,
                        lhs_high: *lhs_high,
                        rhs_low: *rhs_low,
                        rhs_high: *rhs_high,
                    }
                };
                ctx.emit(inst);
                Some(smallvec![ValueRegs::two(
                    dst_low.to_reg(),
                    dst_high.to_reg(),
                )])
            }
            Opcode::Band | Opcode::Bor | Opcode::Bxor
                if ctx.output_ty(ir_inst, 0) == types::I16 =>
            {
                let lhs = ctx.put_input_in_regs(ir_inst, 0);
                let rhs = ctx.put_input_in_regs(ir_inst, 1);
                let dst = ctx.alloc_tmp(types::I16);
                let [lhs_low, lhs_high] = lhs.regs() else {
                    return None;
                };
                let [rhs_low, rhs_high] = rhs.regs() else {
                    return None;
                };
                let [dst_low, dst_high] = dst.regs() else {
                    return None;
                };
                let op = ctx.data(ir_inst).opcode();
                for (dst, lhs, rhs) in [
                    (*dst_low, *lhs_low, *rhs_low),
                    (*dst_high, *lhs_high, *rhs_high),
                ] {
                    ctx.emit(match op {
                        Opcode::Band => Inst::And8 { dst, lhs, rhs },
                        Opcode::Bor => Inst::Or8 { dst, lhs, rhs },
                        Opcode::Bxor => Inst::Xor8 { dst, lhs, rhs },
                        _ => unreachable!(),
                    });
                }
                Some(smallvec![ValueRegs::two(
                    dst_low.to_reg(),
                    dst_high.to_reg(),
                )])
            }
            _ => isle::lower(ctx, self, ir_inst),
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
