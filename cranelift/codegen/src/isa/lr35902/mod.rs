use core::fmt;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::marker::PhantomData;
use cranelift_control::ControlPlane;
use std::string::String;
use target_lexicon::Triple;

use crate::ir;
use crate::isa::Builder as IsaBuilder;
use crate::isa::lr35902::settings as lr35902_settings;
use crate::machinst::{CompiledCode, SigSet, compile};
use crate::settings::{self as shared_settings, Flags};
use crate::{
    CodegenResult, dominator_tree::DominatorTree, ir::Function, machinst::CompiledCodeStencil,
};

use super::{FunctionAlignment, IsaFlagsHashKey, OwnedTargetIsa, TargetIsa};

pub(crate) mod abi;
mod inst;
mod lower;
pub(crate) mod sdcc;
pub mod settings;

pub(crate) struct LR35902Backend {
    triple: Triple,
    flags: Flags,
    lr35902_flags: lr35902_settings::Flags,
}

impl LR35902Backend {
    /// Create a new LR35902 backend with the given (shared) flags.
    fn new_with_flags(
        triple: Triple,
        flags: Flags,
        lr35902_flags: lr35902_settings::Flags,
    ) -> Self {
        Self {
            triple,
            flags,
            lr35902_flags,
        }
    }
}

impl TargetIsa for LR35902Backend {
    fn compile_function(
        &self,
        func: &Function,
        domtree: &DominatorTree,
        regalloc_ctx: &mut regalloc2::Ctx,
        want_disasm: bool,
        ctrl_plane: &mut ControlPlane,
    ) -> CodegenResult<CompiledCodeStencil> {
        let emit_info = inst::EmitInfo::new(self.flags.clone(), self.lr35902_flags.clone());
        let sigs = SigSet::new::<abi::LR35902ABIMachineSpec>(func, &self.flags)?;
        let abi = crate::machinst::abi::Callee::<abi::LR35902ABIMachineSpec>::new(
            func,
            self,
            &self.lr35902_flags,
            &sigs,
        )?;
        let vcode = compile::<Self>(
            func,
            domtree,
            regalloc_ctx,
            self,
            abi,
            emit_info,
            sigs,
            ctrl_plane,
        )?;
        let emit_result = vcode.emit(&regalloc_ctx.output, want_disasm, &self.flags, ctrl_plane)?;
        Ok(CompiledCodeStencil(CompiledCode {
            buffer: emit_result.buffer,
            vcode: emit_result.disasm,
            value_labels_ranges: emit_result.value_labels_ranges,
            bb_starts: emit_result.bb_offsets,
            bb_edges: emit_result.bb_edges,
        }))
    }

    fn flags(&self) -> &Flags {
        &self.flags
    }

    fn isa_flags(&self) -> Vec<shared_settings::Value> {
        self.lr35902_flags.iter().collect()
    }

    fn isa_flags_hash_key(&self) -> IsaFlagsHashKey<'_> {
        IsaFlagsHashKey(self.lr35902_flags.hash_key())
    }

    fn dynamic_vector_bytes(&self, _dyn_ty: crate::ir::Type) -> u32 {
        0
    }

    fn name(&self) -> &'static str {
        "lr35902"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn text_section_builder(
        &self,
        num_labeled_funcs: usize,
    ) -> std::prelude::v1::Box<dyn crate::TextSectionBuilder> {
        Box::new(crate::machinst::MachTextSectionBuilder::<inst::Inst>::new(
            num_labeled_funcs,
        ))
    }

    fn function_alignment(&self) -> FunctionAlignment {
        <inst::Inst as crate::machinst::MachInst>::function_alignment()
    }

    fn page_size_align_log2(&self) -> u8 {
        16
    }

    fn pretty_print_reg(&self, reg: crate::Reg, _size: u8) -> std::string::String {
        format!("{reg:?}")
    }

    fn has_native_fma(&self) -> bool {
        // FMA? On a LR35902? Not likely.
        false
    }

    fn has_round(&self) -> bool {
        // LR35902 lacks floating-point support, so no rounding either.
        false
    }

    fn has_blendv_lowering(&self, _ty: crate::ir::Type) -> bool {
        false
    }

    fn has_x86_pshufb_lowering(&self) -> bool {
        false
    }

    fn has_x86_pmulhrsw_lowering(&self) -> bool {
        false
    }

    fn has_x86_pmaddubsw_lowering(&self) -> bool {
        false
    }

    fn default_argument_extension(&self) -> crate::ir::ArgumentExtension {
        ir::ArgumentExtension::None
    }

    #[cfg(feature = "unwind")]
    fn emit_unwind_info(
        &self,
        _result: &CompiledCode,
        _kind: crate::isa::unwind::UnwindInfoKind,
    ) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
        Ok(None)
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        None
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(
        &self,
        _: crate::machinst::Reg,
    ) -> Result<u16, super::unwind::systemv::RegisterMappingError> {
        unimplemented!("LR35902 does not support unwinding");
    }

    #[cfg(feature = "disas")]
    fn to_capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        todo!()
    }
}

impl fmt::Display for LR35902Backend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LR35902Backend")
            .field("name", &self.name())
            .field("triple", &self.triple())
            .field("flags", &format!("{}", self.flags()))
            .finish()
    }
}

/// Create a new `isa::Builder`.
pub(crate) fn isa_builder(triple: Triple) -> IsaBuilder {
    IsaBuilder {
        triple,
        setup: lr35902_settings::builder(),
        constructor: isa_constructor,
    }
}

fn isa_constructor(
    triple: Triple,
    shared_flags: Flags,
    builder: &shared_settings::Builder,
) -> CodegenResult<OwnedTargetIsa> {
    let isa_flags = lr35902_settings::Flags::new(&shared_flags, builder);
    let backend = LR35902Backend::new_with_flags(triple, shared_flags, isa_flags);
    Ok(backend.wrapped())
}

#[cfg(all(test, feature = "lr35902"))]
mod lowering_tests {
    use super::isa_builder;
    use crate::cursor::Cursor;
    use crate::dominator_tree::DominatorTree;
    use crate::flowgraph::ControlFlowGraph;
    use crate::ir::condcodes::IntCC;
    use crate::ir::{
        AbiParam, ExtFuncData, ExternalName, Function, InstBuilder, Signature, StackSlotData,
        StackSlotKind, UserExternalName, UserFuncName, types,
    };
    use crate::isa::CallConv;
    use crate::machinst::CompiledCodeStencil;
    use crate::settings;
    use alloc::vec::Vec;
    use core::str::FromStr;
    use cranelift_control::ControlPlane;
    use target_lexicon::Triple;

    #[test]
    fn compiles_i8_add_and_return_to_lr35902_machine_code() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 0), Signature::new(CallConv::Fast));
        func.signature
            .returns
            .push(crate::ir::AbiParam::new(types::I8));
        let block = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let lhs = cursor.ins().iconst(types::I8, 5);
        let rhs = cursor.ins().iconst(types::I8, 7);
        let sum = cursor.ins().iadd(lhs, rhs);
        cursor.ins().return_(&[sum]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let shared_flags = settings::Flags::new(settings::builder());
        let backend = isa_builder(triple).finish(shared_flags).unwrap();
        let mut regalloc_ctx = regalloc2::Ctx::default();
        let result: CompiledCodeStencil = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc_ctx,
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();

        assert_eq!(result.0.buffer.data().last(), Some(&0xC9));
    }

    #[test]
    fn compiles_unconditional_jump_to_lr35902_machine_code() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 3), Signature::new(CallConv::Fast));
        let entry = func.dfg.make_block();
        let target = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(entry);
        cursor.ins().jump(target, &[]);
        cursor.insert_block(target);
        cursor.ins().return_(&[]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();

        assert_eq!(result.0.buffer.data(), &[0xC3, 0x03, 0x00, 0xC9]);
    }

    #[test]
    fn compiles_direct_call_with_stack_argument() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 4), Signature::new(CallConv::Fast));
        let callee_sig = Signature {
            params: vec![AbiParam::new(types::I8); 8],
            returns: vec![],
            call_conv: CallConv::Fast,
        };
        let signature = func.dfg.signatures.push(callee_sig);
        let user_name = func.declare_imported_user_function(UserExternalName::new(0, 1));
        let callee = func.import_function(ExtFuncData {
            name: ExternalName::user(user_name),
            signature,
            colocated: true,
            patchable: false,
        });
        let block = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let args = (1..=8)
            .map(|value| cursor.ins().iconst(types::I8, value))
            .collect::<Vec<_>>();
        cursor.ins().call(callee, &args);
        cursor.ins().return_(&[]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();
        let code = result.0.buffer.data();

        assert!(code.contains(&0xCD));
        assert!(code.windows(3).any(|bytes| bytes == [0xF8, 0x00, 0x77]));
    }

    #[test]
    fn loads_incoming_stack_argument_after_return_address() {
        let mut signature = Signature::new(CallConv::Fast);
        signature.params = vec![AbiParam::new(types::I8); 8];
        signature.returns.push(AbiParam::new(types::I8));
        let mut func = Function::with_name_signature(UserFuncName::user(0, 5), signature);
        let block = func.dfg.make_block();
        let params = (0..8)
            .map(|_| func.dfg.append_block_param(block, types::I8))
            .collect::<Vec<_>>();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        cursor.ins().return_(&[params[7]]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();

        assert!(
            result
                .0
                .buffer
                .data()
                .windows(3)
                .any(|bytes| bytes == [0xF8, 0x02, 0x7E])
        );
    }

    #[test]
    fn compiles_i8_sub_and_bitwise_ops_to_lr35902_machine_code() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 2), Signature::new(CallConv::Fast));
        func.signature
            .returns
            .push(crate::ir::AbiParam::new(types::I8));
        let block = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let lhs = cursor.ins().iconst(types::I8, 0x5A);
        let rhs = cursor.ins().iconst(types::I8, 0x0F);
        let sub = cursor.ins().isub(lhs, rhs);
        let and = cursor.ins().band(sub, rhs);
        let or = cursor.ins().bor(and, lhs);
        let xor = cursor.ins().bxor(or, rhs);
        cursor.ins().return_(&[xor]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let mut regalloc_ctx = regalloc2::Ctx::default();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc_ctx,
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();

        assert_eq!(result.0.buffer.data().last(), Some(&0xC9));
    }

    #[test]
    fn compiles_i16_arithmetic_and_bitwise_ops_to_lr35902_machine_code() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 6), Signature::new(CallConv::Fast));
        func.signature.returns.push(AbiParam::new(types::I16));
        let block = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let lhs = cursor.ins().iconst(types::I16, 0x12FF);
        let rhs = cursor.ins().iconst(types::I16, 0x0001);
        let sum = cursor.ins().iadd(lhs, rhs);
        let diff = cursor.ins().isub(sum, lhs);
        let result = cursor.ins().band(diff, rhs);
        cursor.ins().return_(&[result]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();
        let code = result.0.buffer.data();

        assert!(code.iter().any(|byte| (0x88..=0x8F).contains(byte)));
        assert!(code.iter().any(|byte| (0x98..=0x9F).contains(byte)));
        assert_eq!(code.last(), Some(&0xC9));
    }

    #[test]
    fn compiles_i16_stack_address_to_lr35902_machine_code() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 8), Signature::new(CallConv::Fast));
        func.signature.returns.push(AbiParam::new(types::I16));
        let slot =
            func.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 2, 1));
        let block = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let address = cursor.ins().stack_addr(types::I16, slot, 0);
        cursor.ins().return_(&[address]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();

        assert!(
            result
                .0
                .buffer
                .data()
                .windows(2)
                .any(|bytes| bytes[0] == 0xF8)
        );
        assert_eq!(result.0.buffer.data().last(), Some(&0xC9));
    }

    #[test]
    fn compiles_i8_and_i16_memory_accesses() {
        let mut signature = Signature::new(CallConv::Fast);
        signature.params = vec![AbiParam::new(types::I16), AbiParam::new(types::I16)];
        signature.returns.push(AbiParam::new(types::I16));
        let mut func = Function::with_name_signature(UserFuncName::user(0, 7), signature);
        let block = func.dfg.make_block();
        let address = func.dfg.append_block_param(block, types::I16);
        let value = func.dfg.append_block_param(block, types::I16);
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let byte = cursor
            .ins()
            .load(types::I8, crate::ir::MemFlagsData::trusted(), address, 2);
        cursor
            .ins()
            .store(crate::ir::MemFlagsData::trusted(), byte, address, 4);
        let word = cursor
            .ins()
            .load(types::I16, crate::ir::MemFlagsData::trusted(), address, 6);
        cursor
            .ins()
            .store(crate::ir::MemFlagsData::trusted(), value, address, 8);
        cursor.ins().return_(&[word]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();
        let code = result.0.buffer.data();

        assert!(code.windows(2).any(|bytes| bytes == [0x60, 0x69]));
        assert!(code.contains(&0x7E));
        assert!(code.contains(&0x77));
        assert_eq!(code.last(), Some(&0xC9));
    }

    #[test]
    fn compiles_i8_signed_comparison_and_conditional_branch() {
        let mut signature = Signature::new(CallConv::Fast);
        signature.params = vec![AbiParam::new(types::I8), AbiParam::new(types::I8)];
        let mut func = Function::with_name_signature(UserFuncName::user(0, 8), signature);
        let entry = func.dfg.make_block();
        let lhs = func.dfg.append_block_param(entry, types::I8);
        let rhs = func.dfg.append_block_param(entry, types::I8);
        let yes = func.dfg.make_block();
        let no = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(entry);
        let condition = cursor.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
        cursor.ins().brif(condition, yes, &[], no, &[]);
        cursor.insert_block(yes);
        let value = cursor.ins().iconst(types::I8, 1);
        cursor.ins().return_(&[value]);
        cursor.insert_block(no);
        let value = cursor.ins().iconst(types::I8, 0);
        cursor.ins().return_(&[value]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();
        let code = result.0.buffer.data();

        assert!(code.windows(2).any(|bytes| bytes == [0xE6, 0x80]));
        assert!(code.contains(&0xC2));
        assert!(code.contains(&0xC3));
    }

    #[test]
    fn compiles_i16_unsigned_comparison() {
        let mut signature = Signature::new(CallConv::Fast);
        signature.params = vec![AbiParam::new(types::I16), AbiParam::new(types::I16)];
        let mut func = Function::with_name_signature(UserFuncName::user(0, 9), signature);
        let entry = func.dfg.make_block();
        let lhs = func.dfg.append_block_param(entry, types::I16);
        let rhs = func.dfg.append_block_param(entry, types::I16);
        let yes = func.dfg.make_block();
        let no = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(entry);
        let condition = cursor.ins().icmp(IntCC::UnsignedLessThan, lhs, rhs);
        cursor.ins().brif(condition, yes, &[], no, &[]);
        cursor.insert_block(yes);
        cursor.ins().return_(&[]);
        cursor.insert_block(no);
        cursor.ins().return_(&[]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc2::Ctx::default(),
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();
        let code = result.0.buffer.data();

        assert!(code.contains(&0x28));
        assert!(code.contains(&0xC2));
        assert!(code.contains(&0xC3));
    }

    #[test]
    fn compiles_i8_add_with_spills() {
        let mut func =
            Function::with_name_signature(UserFuncName::user(0, 1), Signature::new(CallConv::Fast));
        func.signature
            .returns
            .push(crate::ir::AbiParam::new(types::I8));
        let block = func.dfg.make_block();
        let mut cursor = crate::cursor::FuncCursor::new(&mut func);
        cursor.insert_block(block);
        let mut values = Vec::new();
        for imm in 1..=12 {
            values.push(cursor.ins().iconst(types::I8, imm));
        }
        let mut sum = values[0];
        for value in values.into_iter().skip(1) {
            sum = cursor.ins().iadd(sum, value);
        }
        cursor.ins().return_(&[sum]);
        drop(cursor);

        let cfg = ControlFlowGraph::with_function(&func);
        let domtree = DominatorTree::with_function(&func, &cfg);
        let triple = Triple::from_str("z80-unknown-none").unwrap();
        let backend = isa_builder(triple)
            .finish(settings::Flags::new(settings::builder()))
            .unwrap();
        let mut regalloc_ctx = regalloc2::Ctx::default();
        let result = backend
            .compile_function(
                &func,
                &domtree,
                &mut regalloc_ctx,
                false,
                &mut ControlPlane::default(),
            )
            .unwrap();

        assert!(result.0.buffer.data().contains(&0xF8));
        assert_eq!(result.0.buffer.data().last(), Some(&0xC9));
    }
}
