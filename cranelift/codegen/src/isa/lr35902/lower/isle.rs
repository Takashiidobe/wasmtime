pub mod generated_code;
use generated_code::MInst;

// Types that the generated ISLE code uses via `use super::*`.
use crate::ir::{Inst, condcodes::*, immediates::*, types::*, *};
use crate::isa::lr35902::inst::{regs, *};
use crate::isa::lr35902::*;
use crate::machinst::{
    CallArgList, CallInfo, CallRetList, MachInst, Reg, VCodeConstant, VCodeConstantData,
    abi::{ArgPair, RetPair, StackAMode},
    isle::*,
};
use regalloc2::PReg;

type Unit = ();
type VecArgPair = Vec<ArgPair>;
type VecRetPair = Vec<RetPair>;

pub(crate) struct LR35902IsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    pub backend: &'a B,
}

impl<'a, 'b> LR35902IsleContext<'a, 'b, MInst, LR35902Backend> {
    fn new(lower_ctx: &'a mut Lower<'b, MInst>, backend: &'a LR35902Backend) -> Self {
        Self { lower_ctx, backend }
    }

    pub(crate) fn dfg(&self) -> &crate::ir::DataFlowGraph {
        &self.lower_ctx.f.dfg
    }
}

pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &LR35902Backend,
    inst: Inst,
) -> Option<InstOutput> {
    let mut isle_ctx = LR35902IsleContext::new(lower_ctx, backend);
    todo!("LR35902 CLIF instruction lowering")
}

pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &LR35902Backend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    let mut isle_ctx = LR35902IsleContext::new(lower_ctx, backend);
    todo!("LR35902 CLIF branch lowering")
}
