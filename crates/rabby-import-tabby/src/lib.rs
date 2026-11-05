//! Tabby importer skeleton: detect -> plan -> apply.

use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ImportPlan {
    pub profiles: usize,
    pub skipped: Vec<String>,
}

pub fn detect_config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home).join(".config").join("tabby");
    path.is_dir().then_some(path)
}

pub fn plan(_config_dir: &std::path::Path) -> ImportPlan {
    ImportPlan::default()
}
