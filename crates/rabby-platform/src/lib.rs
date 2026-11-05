//! Platform services. M0 provides a shell catalog; other traits land in v1.x.

use std::path::Path;
use std::process::Command;

use rabby_core::{Shell, ShellType};

pub trait ShellCatalog {
    fn shells(&self) -> Vec<Shell>;
    fn default_shell(&self) -> Option<Shell> {
        self.shells().into_iter().next()
    }
}

pub struct DefaultPlatform;

impl DefaultPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultPlatform {
    fn default() -> Self {
        Self::new()
    }
}

fn shell_from_path(id: &str, path: &str) -> Shell {
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string();
    let mut shell = Shell::new(id.to_string(), name, path.to_string());
    if !cfg!(windows) {
        shell.args.push("-l".to_string());
    }
    shell.shell_type = if cfg!(windows) {
        ShellType::Cmd
    } else {
        ShellType::Unix
    };
    shell
}

fn system_shells() -> Vec<String> {
    let mut shells = Vec::new();
    #[cfg(unix)]
    {
        if let Ok(text) = std::fs::read_to_string("/etc/shells") {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                shells.push(line.to_string());
            }
        }
    }
    shells
}

#[cfg(target_os = "macos")]
fn user_shell() -> Option<String> {
    let user = std::env::var("USER").ok()?;
    let output = Command::new("dscl")
        .args([".", "-read", &format!("/Users/{user}"), "UserShell"])
        .output()
        .ok()?;
    if !output.status.success() {
        return std::env::var("SHELL").ok();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.trim().strip_prefix("UserShell:").map(str::trim))
        .map(ToString::to_string)
        .or_else(|| std::env::var("SHELL").ok())
}

#[cfg(not(target_os = "macos"))]
fn user_shell() -> Option<String> {
    std::env::var("SHELL").ok()
}

impl ShellCatalog for DefaultPlatform {
    fn shells(&self) -> Vec<Shell> {
        let mut shells = Vec::new();
        if let Some(path) = user_shell() {
            shells.push(shell_from_path("default", &path));
        }
        for path in system_shells() {
            if shells.iter().any(|shell| shell.command == path) {
                continue;
            }
            let id = format!("shell-{}", shells.len());
            shells.push(shell_from_path(&id, &path));
        }
        shells
    }

    fn default_shell(&self) -> Option<Shell> {
        let preferred = user_shell();
        let shells = self.shells();
        preferred
            .and_then(|path| shells.iter().find(|shell| shell.command == path).cloned())
            .or_else(|| shells.into_iter().next())
    }
}
