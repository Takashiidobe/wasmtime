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
            Opcode::Icmp if ctx.output_ty(ir_inst, 0) == types::I8 => {
                let condition = ctx.data(ir_inst).cond_code()?;
                let lhs = ctx.put_input_in_regs(ir_inst, 0);
                let rhs = ctx.put_input_in_regs(ir_inst, 1);
                let dst = ctx.alloc_tmp(types::I8).only_reg()?;
                let accumulator = ctx.alloc_tmp(types::I8).only_reg()?;
                let scratch = ctx.alloc_tmp(types::I8).only_reg()?;
                let inst = match ctx.input_ty(ir_inst, 0) {
                    types::I8 => Inst::Compare8 {
                        accumulator,
                        scratch,
                        dst,
                        lhs: lhs.only_reg()?,
                        rhs: rhs.only_reg()?,
                        condition,
                    },
                    types::I16 => {
                        let [lhs_low, lhs_high] = lhs.regs() else {
                            return None;
                        };
                        let [rhs_low, rhs_high] = rhs.regs() else {
                            return None;
                        };
                        Inst::Compare16 {
                            accumulator,
                            scratch,
                            dst,
                            lhs_low: *lhs_low,
                            lhs_high: *lhs_high,
                            rhs_low: *rhs_low,
                            rhs_high: *rhs_high,
                            condition,
                        }
                    }
                    _ => return None,
                };
                ctx.emit(inst);
                Some(smallvec![ValueRegs::one(dst.to_reg())])
            }
            Opcode::Load => {
                let ty = ctx.output_ty(ir_inst, 0);
                let offset = ctx.data(ir_inst).load_store_offset().unwrap_or(0);
                if !(-128..=127).contains(&offset)
                    || ctx.memflags(ir_inst)?.explicit_endianness()
                        == Some(crate::ir::Endianness::Big)
                {
                    return None;
                }
                let address = ctx.put_input_in_regs(ir_inst, 0);
                let [address_low, address_high] = address.regs() else {
                    return None;
                };
                let scratch_h = ctx.alloc_tmp(types::I8).only_reg()?;
                let scratch_l = ctx.alloc_tmp(types::I8).only_reg()?;
                let accumulator = ctx.alloc_tmp(types::I8).only_reg()?;
                match ty {
                    types::I8 => {
                        let dst = ctx.alloc_tmp(types::I8).only_reg()?;
                        ctx.emit(Inst::LoadMem8 {
                            address_low: *address_low,
                            address_high: *address_high,
                            offset,
                            scratch_h,
                            scratch_l,
                            accumulator,
                            dst,
                        });
                        Some(smallvec![ValueRegs::one(dst.to_reg())])
                    }
                    types::I16 => {
                        let dst = ctx.alloc_tmp(types::I16);
                        let [dst_low, dst_high] = dst.regs() else {
                            return None;
                        };
                        ctx.emit(Inst::LoadMem16 {
                            address_low: *address_low,
                            address_high: *address_high,
                            offset,
                            scratch_h,
                            scratch_l,
                            accumulator,
                            dst_low: *dst_low,
                            dst_high: *dst_high,
                        });
                        Some(smallvec![ValueRegs::two(
                            dst_low.to_reg(),
                            dst_high.to_reg(),
                        )])
                    }
                    _ => None,
                }
            }
            Opcode::Store => {
                let ty = ctx.input_ty(ir_inst, 0);
                let offset = ctx.data(ir_inst).load_store_offset().unwrap_or(0);
                if !(-128..=127).contains(&offset)
                    || ctx.memflags(ir_inst)?.explicit_endianness()
                        == Some(crate::ir::Endianness::Big)
                {
                    return None;
                }
                let address = ctx.put_input_in_regs(ir_inst, 1);
                let [address_low, address_high] = address.regs() else {
                    return None;
                };
                let value = ctx.put_input_in_regs(ir_inst, 0);
                let scratch_h = ctx.alloc_tmp(types::I8).only_reg()?;
                let scratch_l = ctx.alloc_tmp(types::I8).only_reg()?;
                let accumulator = ctx.alloc_tmp(types::I8).only_reg()?;
                match ty {
                    types::I8 => {
                        let src = value.only_reg()?;
                        ctx.emit(Inst::StoreMem8 {
                            address_low: *address_low,
                            address_high: *address_high,
                            offset,
                            scratch_h,
                            scratch_l,
                            accumulator,
                            src,
                        });
                        Some(smallvec![])
                    }
                    types::I16 => {
                        let [src_low, src_high] = value.regs() else {
                            return None;
                        };
                        ctx.emit(Inst::StoreMem16 {
                            address_low: *address_low,
                            address_high: *address_high,
                            offset,
                            scratch_h,
                            scratch_l,
                            accumulator,
                            src_low: *src_low,
                            src_high: *src_high,
                        });
                        Some(smallvec![])
                    }
                    _ => None,
                }
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
        match (ctx.data(ir_inst).opcode(), targets) {
            (Opcode::Jump, [target]) => ctx.emit(Inst::jmp_known(*target)),
            (Opcode::Brif, [taken, not_taken]) => {
                let condition = ctx.put_input_in_regs(ir_inst, 0).only_reg()?;
                ctx.emit(Inst::BranchIf {
                    condition,
                    taken: *taken,
                    not_taken: *not_taken,
                });
            }
            _ => return None,
        }
        Some(())
    }

    fn maybe_pinned_reg(&self) -> Option<Reg> {
        None
    }
}
