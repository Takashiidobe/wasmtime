//! LR35902 ABI implementation.

use crate::CodegenResult;
use crate::ir;
use crate::ir::Signature;
use crate::isa;
use crate::isa::lr35902::inst::{Inst, regs};
use crate::isa::lr35902::settings as lr35902_settings;
use crate::machinst::*;
use crate::settings;
use alloc::vec::Vec;
use regalloc2::{MachineEnv, PRegSet};
use smallvec::SmallVec;

/// LR35902-specific ABI behavior
pub struct LR35902ABIMachineSpec;

impl IsaFlags for lr35902_settings::Flags {}

impl ABIMachineSpec for LR35902ABIMachineSpec {
    type I = Inst;
    type F = lr35902_settings::Flags;

    const STACK_ARG_RET_SIZE_LIMIT: u32 = 64 * 1024;

    fn word_bits() -> u32 {
        // The LR35902 has a 16-bit address space.
        16
    }

    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        // LR35902 has no specific stack alignment requirements
        1
    }

    fn compute_arg_locs(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _params: &[ir::AbiParam],
        _args_or_rets: ArgsOrRets,
        _add_ret_area_ptr: bool,
        _args: ArgsAccumulator,
    ) -> CodegenResult<(u32, Option<usize>)> {
        todo!("LR35902 compute_arg_locs")
    }

    fn gen_load_stack(_mem: StackAMode, _into_reg: Writable<Reg>, _ty: ir::Type) -> Self::I {
        todo!("LR35902 gen_load_stack")
    }

    fn gen_store_stack(_mem: StackAMode, _from_reg: Reg, _ty: ir::Type) -> Self::I {
        todo!("LR35902 gen_store_stack")
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: ir::Type) -> Self::I {
        Inst::gen_move(to_reg, from_reg, ty)
    }

    fn gen_extend(
        _to_reg: Writable<Reg>,
        _from_reg: Reg,
        _is_signed: bool,
        _from_bits: u8,
        _to_bits: u8,
    ) -> Self::I {
        todo!("LR35902 gen_extend")
    }

    fn gen_args(_args: Vec<ArgPair>) -> Self::I {
        todo!("LR35902 gen_args")
    }

    fn gen_rets(_rets: Vec<RetPair>) -> Self::I {
        todo!("LR35902 gen_rets")
    }

    fn gen_add_imm(
        _call_conv: isa::CallConv,
        _into_reg: Writable<Reg>,
        _from_reg: Reg,
        _imm: u32,
    ) -> SmallInstVec<Self::I> {
        todo!("LR35902 gen_add_imm")
    }

    fn gen_stack_lower_bound_trap(_limit_reg: Reg) -> SmallInstVec<Self::I> {
        todo!("LR35902 gen_stack_lower_bound_trap")
    }

    fn gen_get_stack_addr(_mem: StackAMode, _into_reg: Writable<Reg>) -> Self::I {
        todo!("LR35902 gen_get_stack_addr")
    }

    fn get_stacklimit_reg(_call_conv: isa::CallConv) -> Reg {
        todo!("LR35902 get_stacklimit_reg")
    }

    fn gen_load_base_offset(
        _into_reg: Writable<Reg>,
        _base: Reg,
        _offset: i32,
        _ty: ir::Type,
    ) -> Self::I {
        todo!("LR35902 gen_load_base_offset")
    }

    fn gen_store_base_offset(_base: Reg, _offset: i32, _from_reg: Reg, _ty: ir::Type) -> Self::I {
        todo!("LR35902 gen_store_base_offset")
    }

    fn gen_sp_reg_adjust(_amount: i32) -> SmallInstVec<Self::I> {
        todo!("LR35902 gen_sp_reg_adjust")
    }

    fn compute_frame_layout(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _sig: &Signature,
        _regs: &[Writable<RealReg>],
        function_calls: FunctionCalls,
        _incoming_args_size: u32,
        _tail_args_size: u32,
        _stackslots_size: u32,
        _fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> FrameLayout {
        // TODO: Implement proper frame layout computation
        FrameLayout {
            word_bytes: Self::word_bytes(),
            function_calls,
            incoming_args_size: 0,
            tail_args_size: 0,
            setup_area_size: 0,
            clobber_size: 0,
            fixed_frame_storage_size: 0,
            outgoing_args_size: 0,
            stackslots_size: 0,
            clobbered_callee_saves: vec![],
        }
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        todo!("LR35902 gen_prologue_frame_setup")
    }

    fn gen_epilogue_frame_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        todo!("LR35902 gen_epilogue_frame_restore")
    }

    fn gen_return(
        _call_conv: isa::CallConv,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        todo!("LR35902 gen_return")
    }

    fn gen_probestack(_insts: &mut SmallInstVec<Self::I>, _frame_size: u32) {
        todo!("LR35902 gen_probestack")
    }

    fn gen_inline_probestack(
        _insts: &mut SmallInstVec<Self::I>,
        _call_conv: isa::CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        todo!("LR35902 gen_inline_probestack")
    }

    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        todo!("LR35902 gen_clobber_save")
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        todo!("LR35902 gen_clobber_restore")
    }

    fn gen_memcpy<F: FnMut(ir::Type) -> Writable<Reg>>(
        _call_conv: isa::CallConv,
        _dst: Reg,
        _src: Reg,
        _size: usize,
        _alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        todo!("LR35902 gen_memcpy")
    }

    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &Self::F,
    ) -> u32 {
        u32::from(rc == RegClass::Int)
    }

    fn get_machine_env(_flags: &settings::Flags, _call_conv: isa::CallConv) -> &MachineEnv {
        use std::sync::OnceLock;

        static MACHINE_ENV: OnceLock<MachineEnv> = OnceLock::new();
        MACHINE_ENV.get_or_init(|| MachineEnv {
            preferred_regs_by_class: [
                regs::allocatable_int_regs(),
                PRegSet::empty(),
                PRegSet::empty(),
            ],
            non_preferred_regs_by_class: [PRegSet::empty(), PRegSet::empty(), PRegSet::empty()],
            scratch_by_class: [None, None, None],
            fixed_stack_slots: vec![],
        })
    }

    fn get_regs_clobbered_by_call(
        _call_conv_of_callee: isa::CallConv,
        _is_exception: bool,
    ) -> PRegSet {
        PRegSet::empty()
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        _specified: ir::ArgumentExtension,
        _location: ABIArgLocation,
    ) -> ir::ArgumentExtension {
        ir::ArgumentExtension::None
    }

    fn retval_temp_reg(_call_conv_of_callee: isa::CallConv) -> Writable<Reg> {
        todo!("LR35902 retval_temp_reg")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_env_allocates_the_seven_lr35902_byte_registers() {
        let flags = settings::Flags::new(settings::builder());
        let env = LR35902ABIMachineSpec::get_machine_env(&flags, isa::CallConv::Fast);
        let allocatable = env.preferred_regs_by_class[RegClass::Int as usize];
        assert_eq!(allocatable.len(), 7);
        for reg in regs::Gpr8::ALL {
            assert!(allocatable.contains(regs::preg(reg)));
        }
        assert!(env.preferred_regs_by_class[RegClass::Float as usize].len() == 0);
        assert!(env.preferred_regs_by_class[RegClass::Vector as usize].len() == 0);
    }

    #[test]
    fn integer_register_values_use_one_byte_per_spill_slot() {
        let flags = lr35902_settings::Flags::new(
            &settings::Flags::new(settings::builder()),
            &lr35902_settings::builder(),
        );
        assert_eq!(
            LR35902ABIMachineSpec::get_number_of_spillslots_for_value(RegClass::Int, 0, &flags,),
            1
        );
    }
}
