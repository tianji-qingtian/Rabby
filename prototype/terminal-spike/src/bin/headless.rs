use std::io::Read;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use wezterm_term::color::ColorPalette;
use wezterm_term::{Terminal, TerminalConfiguration, TerminalSize, UnicodeVersion};

#[derive(Debug)]
struct SpikeConfig {
    scrollback: usize,
}

impl TerminalConfiguration for SpikeConfig {
    fn scrollback_size(&self) -> usize {
        self.scrollback
    }

    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }

    fn unicode_version(&self) -> UnicodeVersion {
        UnicodeVersion::new(11)
    }

    fn enable_kitty_keyboard(&self) -> bool {
        false
    }

    fn enable_kitty_graphics(&self) -> bool {
        true
    }
}

fn term_size(cols: usize, rows: usize) -> TerminalSize {
    TerminalSize {
        rows,
        cols,
        pixel_width: cols * 8,
        pixel_height: rows * 17,
        dpi: 96,
    }
}

fn pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn visible_rows(term: &Terminal) -> Vec<String> {
    let screen = term.screen();
    let top = screen.phys_row(0);
    screen
        .lines_in_phys_range(top..top + screen.physical_rows)
        .iter()
        .map(|l| l.as_str().trim_end().to_string())
        .collect()
}

fn rss_mb() -> f64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return f64::NAN;
        }
        #[cfg(target_os = "macos")]
        {
            usage.ru_maxrss as f64 / (1024.0 * 1024.0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            usage.ru_maxrss as f64 / 1024.0
        }
    }
}

fn spawn_shell(
    command: &str,
    cols: u16,
    rows: u16,
    scrollback: usize,
) -> Result<(
    Terminal,
    Box<dyn MasterPty + Send>,
    Box<dyn portable_pty::Child + Send + Sync>,
    mpsc::Receiver<Vec<u8>>,
)> {
    let pty = native_pty_system();
    let pair = pty
        .openpty(pty_size(cols, rows))
        .context("openpty failed")?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.arg("-lc");
    cmd.arg(command);
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));

    let child = pair
        .slave
        .spawn_command(cmd)
        .context("spawn_command failed")?;
    drop(pair.slave);

    let writer = pair.master.take_writer().context("take_writer failed")?;
    let mut reader = pair
        .master
        .try_clone_reader()
        .context("try_clone_reader failed")?;

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 65536];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let term = Terminal::new(
        term_size(cols as usize, rows as usize),
        Arc::new(SpikeConfig { scrollback }),
        "RabbySpike",
        "0.1.0",
        writer,
    );

    Ok((term, pair.master, child, rx))
}

fn run_vt() -> Result<bool> {
    let mut term = Terminal::new(
        term_size(80, 24),
        Arc::new(SpikeConfig { scrollback: 1000 }),
        "RabbySpike",
        "0.1.0",
        Box::new(Vec::new()),
    );
    let mut ok = true;

    term.advance_bytes("hello[2J[Hworld");
    let rows = visible_rows(&term);
    let pass = rows.get(0).map(|s| s.contains("world")).unwrap_or(false);
    println!("VT clear/home: {} ({:?})", pass, rows.get(0));
    ok &= pass;

    term.advance_bytes("]0;spike-title");
    let pass = term.get_title() == "spike-title";
    println!("VT OSC title: {} ({:?})", pass, term.get_title());
    ok &= pass;

    term.advance_bytes("[?1049hALT-SCREEN");
    let alt = term.is_alt_screen_active();
    term.advance_bytes("[?1049l");
    let back = !term.is_alt_screen_active();
    println!("VT alt screen enter/leave: {} ({alt}, {back})", alt && back);
    ok &= alt && back;

    term.resize(term_size(100, 30));
    let pass = term.screen().physical_cols == 100 && term.screen().physical_rows == 30;
    println!(
        "VT resize model: {} ({}x{})",
        pass,
        term.screen().physical_cols,
        term.screen().physical_rows
    );
    ok &= pass;

    let mut width = Terminal::new(
        term_size(80, 24),
        Arc::new(SpikeConfig { scrollback: 1000 }),
        "RabbySpike",
        "0.1.0",
        Box::new(Vec::new()),
    );
    width.advance_bytes("你好🙂e\u{0301}");
    let text = visible_rows(&width).get(0).cloned().unwrap_or_default();
    let cols: usize = width
        .screen()
        .lines_in_phys_range(width.screen().phys_row(0)..width.screen().phys_row(0) + 1)
        .get(0)
        .map(|line| line.visible_cells().map(|c| c.width()).sum())
        .unwrap_or(0);
    let pass = cols >= 6;
    println!("VT wide/emoji cells: {} (cols={cols}, text={text:?})", pass);
    ok &= pass;

    Ok(ok)
}

fn run_cjk() -> Result<()> {
    let mut term = Terminal::new(
        term_size(80, 24),
        Arc::new(SpikeConfig { scrollback: 1000 }),
        "RabbySpike",
        "0.1.0",
        Box::new(Vec::new()),
    );
    term.advance_bytes("你好世界 🙂 é́ abc
");
    let rows = visible_rows(&term);
    let text = rows.get(0).cloned().unwrap_or_default();
    let cols: usize = term
        .screen()
        .lines_in_phys_range(term.screen().phys_row(0)..term.screen().phys_row(0) + 1)
        .get(0)
        .map(|line| line.visible_cells().map(|c| c.width()).sum())
        .unwrap_or(0);
    println!("CJK: text={text:?}, chars={}, cells={cols}", text.chars().count());
    Ok(())
}

fn run_resize() -> Result<bool> {
    let (mut term, master, mut child, rx) = spawn_shell(
        "trap 'echo WINCH' WINCH; echo READY; sleep 3",
        80,
        24,
        1000,
    )?;

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut seen_ready = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                if s.contains("READY") {
                    seen_ready = true;
                    break;
                }
                term.advance_bytes(&bytes);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    if !seen_ready {
        println!("RESIZE: FAIL (shell did not print READY)");
        let _ = child.kill();
        return Ok(false);
    }

    master.resize(pty_size(100, 30))?;
    term.resize(term_size(100, 30));

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut seen_winch = false;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                if s.contains("WINCH") {
                    seen_winch = true;
                    term.advance_bytes(&bytes);
                    break;
                }
                term.advance_bytes(&bytes);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = child.wait();
    let pass = seen_winch && term.screen().physical_cols == 100;
    println!(
        "RESIZE: {} (SIGWINCH={seen_winch}, term={}x{})",
        pass,
        term.screen().physical_cols,
        term.screen().physical_rows
    );
    Ok(pass)
}

fn run_throughput() -> Result<()> {
    let start = Instant::now();
    let (mut term, _master, mut child, rx) = spawn_shell(
        "yes | head -c 33554432",
        80,
        24,
        25000,
    )?;

    let mut bytes = 0usize;
    let mut first_byte: Option<Duration> = None;
    while let Ok(chunk) = rx.recv() {
        if first_byte.is_none() {
            first_byte = Some(start.elapsed());
        }
        bytes += chunk.len();
        term.advance_bytes(&chunk);
    }
    let elapsed = start.elapsed();
    let _ = child.wait();

    let rows = visible_rows(&term);
    let scrollback = term.screen().scrollback_rows();
    println!("THROUGHPUT");
    println!("  bytes: {bytes} ({:.1} MB)", bytes as f64 / 1_000_000.0);
    println!("  elapsed: {:.3} s", elapsed.as_secs_f64());
    println!(
        "  rate: {:.1} MB/s",
        bytes as f64 / 1_000_000.0 / elapsed.as_secs_f64()
    );
    println!("  first_byte: {first_byte:?}");
    println!("  scrollback_rows: {scrollback} (config 25000)");
    println!("  visible_rows: {} (first={:?})", rows.len(), rows.get(0));
    println!("  rss_max: {:.1} MB", rss_mb());
    Ok(())
}

fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    match mode.as_str() {
        "vt" => {
            let ok = run_vt()?;
            if !ok {
                std::process::exit(1);
            }
        }
        "cjk" => run_cjk()?,
        "resize" => {
            let ok = run_resize()?;
            if !ok {
                std::process::exit(1);
            }
        }
        "throughput" => run_throughput()?,
        "all" => {
            let vt = run_vt()?;
            run_cjk()?;
            let resize = run_resize()?;
            run_throughput()?;
            if !vt || !resize {
                std::process::exit(1);
            }
        }
        other => anyhow::bail!("unknown mode {other}; use vt|cjk|resize|throughput|all"),
    }
    Ok(())
}
