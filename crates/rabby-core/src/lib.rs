//! Domain types shared by the Rabby crates. No UI and no I/O.

pub type ProfileId = String;
pub type SessionId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Unix,
    PowerShell,
    Cmd,
    Cygwin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub shell_type: ShellType,
}

impl Shell {
    pub fn new(id: impl Into<String>, name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
            shell_type: ShellType::Unix,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    SessionOutput(Vec<u8>),
    SessionExit(Option<i32>),
    TitleChanged(String),
}
