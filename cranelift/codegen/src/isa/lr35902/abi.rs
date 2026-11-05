//! LR35902 ABI implementation.

use crate::CodegenResult;
use crate::ir;
use crate::ir::Signature;
use crate::isa;
use crate::isa::lr35902::inst::{Inst, regs};
use crate::isa::lr35902::settings as lr35902_settings;
use crate::machinst::abi::{ABIArg, ABIArgSlot, ABIArgSlotVec};
use crate::machinst::*;
use crate::settings;
use alloc::vec::Vec;
use regalloc2::{MachineEnv, PRegSet};
use smallvec::SmallVec;

/// LR35902-specific ABI behavior
pub struct LR35902ABIMachineSpec;

fn register_order(args_or_rets: ArgsOrRets) -> [regs::Gpr8; 7] {
    match args_or_rets {
        ArgsOrRets::Args => [
            regs::Gpr8::B,
            regs::Gpr8::C,
            regs::Gpr8::D,
            regs::Gpr8::E,
            regs::Gpr8::H,
            regs::Gpr8::L,
            regs::Gpr8::A,
        ],
        ArgsOrRets::Rets => [
            regs::Gpr8::A,
            regs::Gpr8::B,
            regs::Gpr8::C,
            regs::Gpr8::D,
            regs::Gpr8::E,
            regs::Gpr8::H,
            regs::Gpr8::L,
        ],
    }
}

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
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator,
    ) -> CodegenResult<(u32, Option<usize>)> {
        if add_ret_area_ptr {
            return Err(crate::CodegenError::Unsupported(
                "LR35902 does not support indirect return areas yet".into(),
            ));
        }

        let mut next_reg = 0;
        let mut next_stack = 0;
        let reg_order = register_order(args_or_rets);
        for param in params {
            if matches!(param.purpose, ir::ArgumentPurpose::StructArgument(_)) {
                return Err(crate::CodegenError::Unsupported(
                    "LR35902 struct arguments are not supported yet".into(),
                ));
            }

            let (classes, part_types) = Inst::rc_for_type(&param.value_type)?;
            let mut slots = ABIArgSlotVec::new();
            for (class, ty) in classes.iter().zip(part_types) {
                if *class != RegClass::Int {
                    return Err(crate::CodegenError::Unsupported(
                        "LR35902 ABI only supports integer register classes".into(),
                    ));
                }
                if next_reg < reg_order.len() {
                    let reg = Reg::from_real_reg(regs::preg(reg_order[next_reg]))
                        .to_real_reg()
                        .unwrap();
                    slots.push(ABIArgSlot::Reg {
                        reg,
                        ty: *ty,
                        extension: param.extension,
                    });
                    next_reg += 1;
                } else if args_or_rets == ArgsOrRets::Args {
                    slots.push(ABIArgSlot::Stack {
                        offset: i64::from(next_stack),
                        ty: *ty,
                        extension: param.extension,
                    });
                    next_stack += ty.bytes();
                } else {
                    return Err(crate::CodegenError::Unsupported(
                        "LR35902 stack return values are not supported".into(),
                    ));
                }
            }
            args.push(ABIArg::Slots {
                slots,
                purpose: param.purpose,
            });
        }
        Ok((next_stack, None))
    }

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: ir::Type) -> Self::I {
        assert_eq!(ty, ir::types::I8);
        Inst::LoadStack8 {
            dst: into_reg,
            mode: mem,
        }
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: ir::Type) -> Self::I {
        assert_eq!(ty, ir::types::I8);
        Inst::StoreStack8 {
            src: from_reg,
            mode: mem,
        }
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

    fn gen_args(args: Vec<ArgPair>) -> Self::I {
        Inst::Args { args }
    }

    fn gen_rets(rets: Vec<RetPair>) -> Self::I {
        Inst::Rets { rets }
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

    fn gen_sp_reg_adjust(mut amount: i32) -> SmallInstVec<Self::I> {
        let mut insts = SmallInstVec::new();
        while amount != 0 {
            let step = amount.clamp(i8::MIN as i32, i8::MAX as i32);
            insts.push(Inst::AdjustSP { amount: step as i8 });
            amount -= step;
        }
        insts
    }

    fn maximum_frame_size() -> u32 {
        128
    }

    fn compute_frame_layout(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _sig: &Signature,
        _regs: &[Writable<RealReg>],
        function_calls: FunctionCalls,
        incoming_args_size: u32,
        tail_args_size: u32,
        stackslots_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        // TODO: Implement proper frame layout computation
        FrameLayout {
            word_bytes: Self::word_bytes(),
            function_calls,
            incoming_args_size,
            tail_args_size,
            setup_area_size: 0,
            clobber_size: 0,
            fixed_frame_storage_size,
            outgoing_args_size,
            stackslots_size,
            clobbered_callee_saves: vec![],
        }
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        SmallInstVec::new()
    }

    fn gen_epilogue_frame_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        SmallInstVec::new()
    }

    fn gen_return(
        _call_conv: isa::CallConv,
        _isa_flags: &Self::F,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        smallvec::smallvec![Inst::Ret]
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
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();
        insts.extend(Self::gen_sp_reg_adjust(
            -((frame_layout.fixed_frame_storage_size + frame_layout.outgoing_args_size) as i32),
        ));
        insts
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();
        insts.extend(Self::gen_sp_reg_adjust(
            (frame_layout.fixed_frame_storage_size + frame_layout.outgoing_args_size) as i32,
        ));
        insts
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
        regs::allocatable_int_regs()
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        _specified: ir::ArgumentExtension,
        _location: ABIArgLocation,
    ) -> ir::ArgumentExtension {
        ir::ArgumentExtension::None
    }

    fn retval_temp_reg(_call_conv_of_callee: isa::CallConv) -> Writable<Reg> {
        Writable::from_reg(regs::a())
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
    fn byte_abi_assigns_arguments_and_returns_in_documented_register_order() {
        assert_eq!(
            register_order(ArgsOrRets::Args),
            [
                regs::Gpr8::B,
                regs::Gpr8::C,
                regs::Gpr8::D,
                regs::Gpr8::E,
                regs::Gpr8::H,
                regs::Gpr8::L,
                regs::Gpr8::A,
            ]
        );
        assert_eq!(
            register_order(ArgsOrRets::Rets),
            [
                regs::Gpr8::A,
                regs::Gpr8::B,
                regs::Gpr8::C,
                regs::Gpr8::D,
                regs::Gpr8::E,
                regs::Gpr8::H,
                regs::Gpr8::L,
            ]
        );
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
