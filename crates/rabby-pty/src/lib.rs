//! Local PTY transport. One reader thread per session; no UI types.

use std::io::{Read, Write};

use anyhow::{Context, Result};
use async_channel::{Receiver, bounded};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use rabby_core::Shell;

pub struct PtySession {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
    pub reader: Receiver<Vec<u8>>,
    writer: Option<Box<dyn Write + Send>>,
}

impl PtySession {
    pub fn spawn(shell: &Shell, cols: u16, rows: u16) -> Result<Self> {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let mut command = CommandBuilder::new(&shell.command);
        for arg in &shell.args {
            command.arg(arg);
        }
        command.env("TERM", "xterm-256color");
        if let Some(home) = std::env::var_os("HOME") {
            command.cwd(home);
        }

        let child = pair.slave.spawn_command(command).context("spawn shell")?;
        drop(pair.slave);

        let writer = pair.master.take_writer().context("pty writer")?;
        let mut reader = pair.master.try_clone_reader().context("pty reader")?;
        let (tx, rx) = bounded(256);
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send_blocking(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master: pair.master,
            child,
            reader: rx,
            writer: Some(writer),
        })
    }

    pub fn take_writer(&mut self) -> Option<Box<dyn Write + Send>> {
        self.writer.take()
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("pty resize")
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
