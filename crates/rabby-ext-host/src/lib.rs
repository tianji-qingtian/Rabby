//! Extension host skeleton (v1.x). gpui-shell adapter lands after M4.

pub use rabby_ext_api::Manifest;

pub struct ExtensionHost;

impl ExtensionHost {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExtensionHost {
    fn default() -> Self {
        Self::new()
    }
}
