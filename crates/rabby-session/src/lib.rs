//! Session orchestration skeleton: local PTY + TerminalCore.

use rabby_core::Shell;
use rabby_pty::PtySession;
use rabby_terminal::TerminalCore;

pub struct LocalSession {
    pub core: TerminalCore,
    pub pty: PtySession,
}

impl LocalSession {
    pub fn spawn(shell: &Shell, cols: u16, rows: u16) -> anyhow::Result<Self> {
        let mut pty = PtySession::spawn(shell, cols, rows)?;
        let writer = pty.take_writer().expect("pty writer");
        let core = TerminalCore::new(cols as usize, rows as usize, 25_000, writer);
        Ok(Self { core, pty })
    }
}
