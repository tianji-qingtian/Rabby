//! Minimal GPUI terminal view: TerminalPane owns TerminalCore + PtySession.

use gpui_kit::component::*;
use gpui_kit::*;
use rabby_pty::PtySession;
use rabby_terminal::{KeyCode, KeyModifiers, TerminalCore};

const FONT_SIZE: f32 = 14.0;
const CELL_W: f32 = 8.4;
const LINE_H: f32 = 17.0;

pub struct TerminalElement;

impl TerminalElement {
    pub fn render(rows: Vec<SharedString>, line_height: f32) -> impl IntoElement {
        let mut lines = v_flex().size_full();
        for row in rows {
            lines = lines.child(div().h(px(line_height)).overflow_hidden().child(row));
        }
        lines
    }
}

pub struct PaneActivated;

impl EventEmitter<PaneActivated> for TerminalPane {}

pub struct TerminalPane {
    core: TerminalCore,
    pty: PtySession,
    focus_handle: FocusHandle,
    rows: Vec<SharedString>,
    cols: usize,
    rows_count: usize,
}

impl TerminalPane {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        core: TerminalCore,
        pty: PtySession,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let mut pane = Self {
            core,
            pty,
            focus_handle,
            rows: Vec::new(),
            cols: 80,
            rows_count: 24,
        };
        pane.refresh_rows();
        window.focus(&pane.focus_handle, cx);

        let reader = pane.pty.reader.clone();
        cx.spawn(async move |this, cx| {
            while let Ok(chunk) = reader.recv().await {
                let result = this.update(cx, |this, cx| {
                    this.core.feed(&chunk);
                    this.refresh_rows();
                    cx.notify();
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        pane
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus_handle, cx);
    }

    fn refresh_rows(&mut self) {
        self.rows = self
            .core
            .visible_rows()
            .into_iter()
            .map(SharedString::from)
            .collect();
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        if cols == self.cols && rows == self.rows_count {
            return;
        }
        self.cols = cols;
        self.rows_count = rows;
        self.core.resize(cols, rows);
        let _ = self.pty.resize(cols as u16, rows as u16);
        self.refresh_rows();
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        if ks.modifiers.platform {
            return;
        }
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

        if let Some(ch) = &ks.key_char
            && !ch.is_empty()
        {
            for c in ch.chars() {
                let _ = self.core.key_down(KeyCode::Char(c), mods);
            }
            cx.notify();
            return;
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
        let _ = self.core.key_down(key, mods);
        cx.notify();
    }
}

impl Render for TerminalPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let cols = (((viewport.width.as_f32() - 16.0) / CELL_W).floor() as usize).max(20);
        let rows = (((viewport.height.as_f32() - 16.0) / LINE_H).floor() as usize).max(5);
        self.resize(cols, rows);

        let element = TerminalElement::render(self.rows.clone(), LINE_H);
        div()
            .size_full()
            .bg(rgb(0x101014))
            .text_color(rgb(0xdddddd))
            .font_family("Menlo")
            .text_size(px(FONT_SIZE))
            .track_focus(&self.focus_handle)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus_handle, cx);
                    cx.emit(PaneActivated);
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key(event, cx);
            }))
            .child(element)
    }
}
