//! Fixtures and mocks shared by tests.

pub fn temp_dir_name(prefix: &str) -> String {
    format!("{prefix}-{}", std::process::id())
}
