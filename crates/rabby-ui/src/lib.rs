//! Workspace/AppShell skeleton. M0 hosts one terminal pane; splits/tabs follow.

use gpui_kit::*;
use rabby_terminal_view::TerminalPane;

pub struct Workspace {
    pane: Entity<TerminalPane>,
}

impl Workspace {
    pub fn new(pane: Entity<TerminalPane>) -> Self {
        Self { pane }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.pane.clone())
    }
}
