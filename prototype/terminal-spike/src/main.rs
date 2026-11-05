use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use gpui_kit::component::*;
use gpui_kit::*;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use smol::Timer;
use wezterm_term::color::ColorPalette;
use wezterm_term::input::{KeyCode, KeyModifiers};
use wezterm_term::{Terminal, TerminalConfiguration, TerminalSize, UnicodeVersion};

const FONT_SIZE: f32 = 14.0;
const CELL_W: f32 = 8.4;
const LINE_H: f32 = 17.0;
const SCROLLBACK: usize = 25_000;

#[derive(Debug)]
struct SpikeConfig;

impl TerminalConfiguration for SpikeConfig {
    fn scrollback_size(&self) -> usize {
        SCROLLBACK
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
        pixel_width: cols * CELL_W as usize,
        pixel_height: rows * LINE_H as usize,
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

fn spawn_shell(
    cols: u16,
    rows: u16,
) -> anyhow::Result<(
    Terminal,
    Box<dyn MasterPty + Send>,
    Box<dyn portable_pty::Child + Send + Sync>,
    mpsc::Receiver<Vec<u8>>,
)> {
    let pty = native_pty_system();
    let pair = pty.openpty(pty_size(cols, rows))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let shell_for_log = shell.clone();
    let mut cmd = CommandBuilder::new(shell);
    if let Ok(script) = std::env::var("SPIKE_CMD") {
        cmd.arg("-lc");
        cmd.arg(script);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
    eprintln!("spawn_shell: shell={shell_for_log}, SPIKE_CMD={:?}", std::env::var("SPIKE_CMD"));
    let child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let writer = pair.master.take_writer()?;
    let mut reader = pair.master.try_clone_reader()?;
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 65536];
        let mut first = true;
        loop {
            match reader.read(&mut buf) {
                Ok(0) => { eprintln!("reader: EOF"); break; }
                Ok(n) => {
                    if first { eprintln!("reader: first {n} bytes"); first = false; }
                    if tx.send(buf[..n].to_vec()).is_err() {
                        eprintln!("reader: channel closed");
                        break;
                    }
                }
                Err(e) => { eprintln!("reader: error {e}"); break; }
            }
        }
    });

    let term = Terminal::new(
        term_size(cols as usize, rows as usize),
        Arc::new(SpikeConfig),
        "RabbySpike",
        "0.1.0",
        writer,
    );
    Ok((term, pair.master, child, rx))
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

struct TerminalView {
    term: Terminal,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    rx: mpsc::Receiver<Vec<u8>>,
    focus_handle: FocusHandle,
    rows_text: Vec<SharedString>,
    cols: usize,
    rows: usize,
    started: Instant,
    first_frame: bool,
    frames: u64,
    render_ms_sum: f64,
    render_ms_max: f64,
    bytes: u64,
    last_report: Instant,
    last_report_bytes: u64,
}

impl TerminalView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (term, master, child, rx) = spawn_shell(80, 24).expect("spawn shell");
        let focus_handle = cx.focus_handle();
        let mut view = Self {
            term,
            master,
            child,
            rx,
            focus_handle,
            rows_text: Vec::new(),
            cols: 80,
            rows: 24,
            started: Instant::now(),
            first_frame: false,
            frames: 0,
            render_ms_sum: 0.0,
            render_ms_max: 0.0,
            bytes: 0,
            last_report: Instant::now(),
            last_report_bytes: 0,
        };
        view.refresh_rows();
        window.focus(&view.focus_handle, cx);

        cx.spawn(async move |this, cx| {
            let mut ticks = 0u64;
            loop {
                Timer::after(Duration::from_millis(8)).await;
                ticks += 1;
                let result = this.update(cx, |this, cx| {
                    let mut changed = false;
                    let mut got = 0u64;
                    // Cap per-tick work so frame painting is not starved by a
                    // firehose of PTY output. The prototype's first version
                    // drained the whole channel and blocked frames for seconds.
                    let mut budget = 1_000_000usize;
                    while budget > 0 {
                        match this.rx.try_recv() {
                            Ok(chunk) => {
                                budget = budget.saturating_sub(chunk.len());
                                got += chunk.len() as u64;
                                this.term.advance_bytes(&chunk);
                                changed = true;
                            }
                            Err(_) => break,
                        }
                    }
                    if changed {
                        this.bytes += got;
                        this.refresh_rows();
                        cx.notify();
                    }
                    let now = Instant::now();
                    if now.duration_since(this.last_report) >= Duration::from_secs(1) {
                        let dt = now.duration_since(this.last_report).as_secs_f64();
                        let db = this.bytes - this.last_report_bytes;
                        this.last_report = now;
                        this.last_report_bytes = this.bytes;
                        println!(
                            "feed: {:.1} MB/s total={} MB rows={}",
                            db as f64 / 1e6 / dt,
                            this.bytes / 1_000_000,
                            this.rows_text.len()
                        );
                    }
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        if std::env::var("SPIKE_ANIM").is_ok() {
            cx.spawn(async move |this, cx| {
                let mut n = 0u64;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let result = this.update(cx, |this, cx| {
                        n += 1;
                        this.rows_text = (0..this.rows)
                            .map(|r| {
                                SharedString::from(format!(
                                    "frame {n:06} row {r:02}  你好世界 🙂 terminal spike"
                                ))
                            })
                            .collect();
                        cx.notify();
                    });
                    if result.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }

        view
    }

    fn refresh_rows(&mut self) {
        let rows: Vec<SharedString> = {
            let screen = self.term.screen();
            let top = screen.phys_row(0);
            screen
                .lines_in_phys_range(top..top + screen.physical_rows)
                .iter()
                .map(|l| SharedString::from(l.as_str().trim_end().to_string()))
                .collect()
        };
        self.rows_text = rows;
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.term.resize(term_size(cols, rows));
        let _ = self
            .master
            .resize(pty_size(cols as u16, rows as u16));
        self.refresh_rows();
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        let mut mods = KeyModifiers::NONE;
        if ks.modifiers.control {
            mods |= KeyModifiers::CTRL;
        }
        if ks.modifiers.alt {
            mods |= KeyModifiers::ALT;
        }
        if ks.modifiers.shift {
            mods |= KeyModifiers::SHIFT;
        }

        if let Some(ch) = &ks.key_char {
            if !ch.is_empty() {
                for c in ch.chars() {
                    let _ = self.term.key_down(KeyCode::Char(c), mods);
                }
                cx.notify();
                return;
            }
        }

        let key = match ks.key.as_str() {
            "enter" => KeyCode::Enter,
            "backspace" => KeyCode::Backspace,
            "tab" => KeyCode::Tab,
            "escape" => KeyCode::Escape,
            "left" => KeyCode::LeftArrow,
            "right" => KeyCode::RightArrow,
            "up" => KeyCode::UpArrow,
            "down" => KeyCode::DownArrow,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "delete" => KeyCode::Delete,
            "insert" => KeyCode::Insert,
            _ => return,
        };
        let _ = self.term.key_down(key, mods);
        cx.notify();
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let cols = (((viewport.width.as_f32() - 16.0) / CELL_W).floor() as usize).max(20);
        let rows = (((viewport.height.as_f32() - 16.0) / LINE_H).floor() as usize).max(5);
        self.resize(cols, rows);

        let t0 = Instant::now();
        let mut lines = v_flex().size_full().p(px(8.0));
        for row in &self.rows_text {
            lines = lines.child(div().h(px(LINE_H)).overflow_hidden().child(row.clone()));
        }

        let view = div()
            .size_full()
            .bg(rgb(0x101014))
            .text_color(rgb(0xdddddd))
            .font_family("Menlo")
            .text_size(px(FONT_SIZE))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key(event, cx);
            }))
            .child(lines);

        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.frames += 1;
        self.render_ms_sum += elapsed_ms;
        self.render_ms_max = self.render_ms_max.max(elapsed_ms);
        if !self.first_frame {
            self.first_frame = true;
            println!(
                "first_frame: render_ms={:.2}, since_start_ms={:.1}",
                elapsed_ms,
                self.started.elapsed().as_secs_f64() * 1000.0
            );
        }
        if self.frames % 30 == 0 {
            println!(
                "frames={} avg_render_ms={:.2} max_render_ms={:.2} bytes={}",
                self.frames,
                self.render_ms_sum / 30.0,
                self.render_ms_max,
                self.bytes
            );
            self.render_ms_sum = 0.0;
            self.render_ms_max = 0.0;
        }
        view
    }
}

fn main() {
    let app = gpui_kit::application();
    app.run(move |cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(960.0), px(640.0)), cx)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| TerminalView::new(window, cx))
        })
        .expect("open window");
    });
}
