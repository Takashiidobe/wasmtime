use core::fmt;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::marker::PhantomData;
use std::string::String;
use cranelift_control::ControlPlane;
use target_lexicon::Triple;

use crate::ir;
use crate::isa::Builder as IsaBuilder;
use crate::isa::lr35902::settings as lr35902_settings;
use crate::machinst::CompiledCode;
use crate::settings::{self as shared_settings, Flags};
use crate::{
    CodegenResult, dominator_tree::DominatorTree, ir::Function, machinst::CompiledCodeStencil,
};

use super::{FunctionAlignment, IsaFlagsHashKey, OwnedTargetIsa, TargetIsa};

pub mod settings;
pub(crate) mod abi;
pub(crate) mod sdcc;
mod lower;
mod inst;

pub(crate) struct LR35902Backend {
    triple: Triple,
    flags: Flags,
    lr35902_flags: lr35902_settings::Flags,
}

impl LR35902Backend {
    /// Create a new LR35902 backend with the given (shared) flags.
    fn new_with_flags(triple: Triple, flags: Flags, lr35902_flags: lr35902_settings::Flags) -> Self {
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
        _func: &Function,
        _domtree: &DominatorTree,
        _regalloc_ctx: &mut regalloc2::Ctx,
        _want_disasm: bool,
        _ctrl_plane: &mut ControlPlane,
    ) -> CodegenResult<CompiledCodeStencil> {
        Err(crate::CodegenError::Unsupported(
            "LR35902 function lowering is not implemented".into(),
        ))
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
        unimplemented!("LR35902 does not support unwinding");
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
