use crate::cdsl::{isa::TargetIsa, settings::SettingGroupBuilder};

pub(crate) fn define() -> TargetIsa {
    TargetIsa::new("lr35902", SettingGroupBuilder::new("lr35902").build())
}
