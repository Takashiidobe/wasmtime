pub mod generated_code;
use generated_code::{Context, MInst};

// Types that the generated ISLE code uses via `use super::*`.
use crate::ir::{Inst, condcodes::*, immediates::*, types::*, *};
use crate::isa::lr35902::inst::{regs, *};
use crate::isa::lr35902::*;
use crate::machinst::{
    CallArgList, CallInfo, CallRetList, MachInst, Reg, VCodeConstant, VCodeConstantData,
    abi::{ArgPair, RetPair, StackAMode},
    isle::*,
};
use alloc::boxed::Box;
use regalloc2::PReg;

type Unit = ();
type BoxCallInfo = alloc::boxed::Box<CallInfo<ExternalName>>;
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

impl Context for LR35902IsleContext<'_, '_, MInst, LR35902Backend> {
    isle_lower_prelude_methods!();

    fn emit(&mut self, inst: &MInst) -> Unit {
        self.lower_ctx.emit(inst.clone());
    }

    fn jmp_known(&mut self, dst: MachLabel) -> generated_code::SideEffectNoResult {
        generated_code::SideEffectNoResult::Inst {
            inst: MInst::JumpKnown { dst },
        }
    }

    fn gen_call_info(
        &mut self,
        sig: Sig,
        dest: ExternalName,
        uses: CallArgList,
        defs: CallRetList,
        try_call_info: Option<TryCallInfo>,
        patchable: bool,
    ) -> BoxCallInfo {
        let stack_ret_space = self.lower_ctx.sigs()[sig].sized_stack_ret_space();
        let stack_arg_space = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_outgoing_args_size(stack_ret_space + stack_arg_space);
        Box::new(
            self.lower_ctx
                .gen_call_info(sig, dest, uses, defs, try_call_info, patchable),
        )
    }
}

pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &LR35902Backend,
    inst: Inst,
) -> Option<InstOutput> {
    let mut isle_ctx = LR35902IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &LR35902Backend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    let mut isle_ctx = LR35902IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
}
