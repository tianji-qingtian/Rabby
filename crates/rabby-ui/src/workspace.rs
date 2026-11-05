use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::*;
use gpui_kit::*;
use rabby_core::{Shell, ShellType};
use rabby_platform::{DefaultPlatform, ShellCatalog};
use rabby_session::LocalSession;
use rabby_terminal_view::{PaneActivated, TerminalPane};

use crate::pane_tree::{PaneNode, SplitDirection};

fn fallback_shell() -> Shell {
    let command = if cfg!(windows) { "cmd.exe" } else { "/bin/sh" };
    let mut shell = Shell::new("fallback", command, command);
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

struct Tab {
    id: u64,
    title: SharedString,
    active_pane: Entity<TerminalPane>,
    tree: PaneNode,
    maximized: bool,
}

#[derive(Clone)]
struct TabDrag {
    id: u64,
    title: SharedString,
}

impl Render for TabDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .bg(rgb(0x2a2a33))
            .text_color(rgb(0xffffff))
            .child(self.title.clone())
    }
}

pub struct Workspace {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    rename_target: Option<u64>,
    rename_input: Entity<InputState>,
    _rename_subscription: Subscription,
    pane_subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let rename_input = cx.new(|cx| InputState::new(window, cx).placeholder("Tab title"));
        let subscription = cx.subscribe_in(
            &rename_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_rename(window, cx);
                }
            },
        );
        let pane = Self::spawn_pane(window, cx);
        let initial_pane = pane.clone();
        let mut workspace = Self {
            tabs: vec![Tab {
                id: 0,
                title: SharedString::from("Terminal 1"),
                active_pane: pane.clone(),
                tree: PaneNode::Leaf(pane),
                maximized: false,
            }],
            active: 0,
            next_id: 1,
            rename_target: None,
            rename_input,
            _rename_subscription: subscription,
            pane_subscriptions: Vec::new(),
        };
        workspace.subscribe_pane(initial_pane, cx);
        workspace.focus_active(window, cx);
        workspace
    }

    fn spawn_pane(window: &mut Window, cx: &mut App) -> Entity<TerminalPane> {
        let shell = DefaultPlatform::new()
            .default_shell()
            .unwrap_or_else(fallback_shell);
        let session = LocalSession::spawn(&shell, 80, 24).expect("spawn local session");
        let LocalSession { core, pty } = session;
        cx.new(|cx| TerminalPane::new(window, cx, core, pty))
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn subscribe_pane(&mut self, pane: Entity<TerminalPane>, cx: &mut Context<Self>) {
        let subscription = cx.subscribe(&pane, |this, pane, _: &PaneActivated, cx| {
            if let Some(tab) = this.tabs.get_mut(this.active)
                && let Some(active) = tab.tree.find(pane.entity_id())
            {
                tab.active_pane = active;
                cx.notify();
            }
        });
        self.pane_subscriptions.push(subscription);
    }

    fn focus_active(&self, window: &mut Window, cx: &mut App) {
        if let Some(tab) = self.active_tab() {
            tab.active_pane
                .update(cx, |pane, cx| pane.focus(window, cx));
        }
    }

    fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        let pane = Self::spawn_pane(window, cx);
        let tab = Tab {
            id,
            title: SharedString::from(format!("Terminal {}", self.tabs.len() + 1)),
            active_pane: pane.clone(),
            tree: PaneNode::Leaf(pane.clone()),
            maximized: false,
        };
        self.tabs.push(tab);
        self.subscribe_pane(pane, cx);
        self.active = self.tabs.len() - 1;
        self.focus_active(window, cx);
        cx.notify();
    }

    fn close_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        if self.rename_target == Some(id) {
            self.rename_target = None;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            let pane = Self::spawn_pane(window, cx);
            self.tabs.push(Tab {
                id: 0,
                title: SharedString::from("Terminal 1"),
                active_pane: pane.clone(),
                tree: PaneNode::Leaf(pane),
                maximized: false,
            });
            self.active = 0;
        } else if index < self.active {
            self.active -= 1;
        } else if index == self.active {
            self.active = index.min(self.tabs.len() - 1);
        }
        self.focus_active(window, cx);
        cx.notify();
    }

    fn activate(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.active = index;
            self.focus_active(window, cx);
            cx.notify();
        }
    }

    fn reorder_tab(&mut self, drag_id: u64, target_id: u64) {
        if drag_id == target_id {
            return;
        }
        let Some(from) = self.tabs.iter().position(|tab| tab.id == drag_id) else {
            return;
        };
        let Some(target) = self.tabs.iter().position(|tab| tab.id == target_id) else {
            return;
        };
        let active_id = self.tabs[self.active].id;
        let tab = self.tabs.remove(from);
        let insert_at = if from < target { target - 1 } else { target };
        self.tabs.insert(insert_at, tab);
        self.active = self
            .tabs
            .iter()
            .position(|tab| tab.id == active_id)
            .unwrap_or(0);
    }

    fn start_rename(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.iter().find(|tab| tab.id == id) else {
            return;
        };
        let title = tab.title.clone();
        self.rename_target = Some(id);
        let input = self.rename_input.clone();
        input.update(cx, |state, cx| {
            state.set_value(title, window, cx);
            state.select_all(window, cx);
        });
        let handle = input.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        cx.notify();
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.rename_target.take() else {
            return;
        };
        let title = self.rename_input.read(cx).value();
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id)
            && !title.is_empty()
        {
            tab.title = title;
        }
        self.focus_active(window, cx);
        cx.notify();
    }

    fn split_active(
        &mut self,
        direction: SplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let new_pane = Self::spawn_pane(window, cx);
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        let active = tab.active_pane.entity_id();
        if tab.tree.split_active(active, new_pane.clone(), direction) {
            tab.active_pane = new_pane.clone();
            tab.maximized = false;
            self.subscribe_pane(new_pane, cx);
            self.focus_active(window, cx);
            cx.notify();
        }
    }

    fn toggle_maximize(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.maximized = !tab.maximized;
            cx.notify();
        }
    }

    fn handle_shortcut(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.rename_target.is_some() {
            if event.keystroke.key == "enter" && event.keystroke.modifiers.platform {
                self.commit_rename(window, cx);
                return;
            }
            if event.keystroke.key == "r" && event.keystroke.modifiers.platform {
                self.commit_rename(window, cx);
                return;
            }
            if event.keystroke.key == "escape" {
                self.rename_target = None;
                self.focus_active(window, cx);
                cx.notify();
                return;
            }
        }
        let ks = &event.keystroke;
        if !ks.modifiers.platform {
            return;
        }
        match ks.key.as_str() {
            "t" => self.new_tab(window, cx),
            "w" => {
                if let Some(tab) = self.active_tab() {
                    let id = tab.id;
                    self.close_tab(id, window, cx);
                }
            }
            "d" => {
                if ks.modifiers.shift {
                    self.split_active(SplitDirection::Vertical, window, cx);
                } else {
                    self.split_active(SplitDirection::Horizontal, window, cx);
                }
            }
            "r" => {
                if let Some(tab) = self.active_tab() {
                    let id = tab.id;
                    self.start_rename(id, window, cx);
                }
            }
            "m" if ks.modifiers.shift => self.toggle_maximize(cx),
            _ => {}
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tab_bar = h_flex()
            .h(px(34.0))
            .items_center()
            .px_2()
            .gap_1()
            .bg(rgb(0x18181f))
            .border_b_1()
            .border_color(rgb(0x2a2a33));

        for (index, tab) in self.tabs.iter().enumerate() {
            let id = tab.id;
            let title = tab.title.clone();
            let active = index == self.active;
            tab_bar = tab_bar.child(
                div()
                    .id(("tab", id))
                    .h_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .bg(if active { rgb(0x101014) } else { rgb(0x18181f) })
                    .text_color(if active { rgb(0xffffff) } else { rgb(0xaaaaaa) })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate(id, window, cx);
                    }))
                    .on_double_click(cx.listener(move |this, _, window, cx| {
                        this.start_rename(id, window, cx);
                    }))
                    .on_drag(
                        TabDrag {
                            id,
                            title: title.clone(),
                        },
                        |drag, _, _, cx| {
                            cx.stop_propagation();
                            cx.new(|_| drag.clone())
                        },
                    )
                    .on_drop(cx.listener(move |this, drag: &TabDrag, _window, cx| {
                        this.reorder_tab(drag.id, id);
                        cx.notify();
                    }))
                    .child(div().max_w(px(160.0)).overflow_hidden().child(title))
                    .child(
                        div()
                            .id(("tab-close", id))
                            .px_1()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.close_tab(id, window, cx);
                            }))
                            .child("×"),
                    ),
            );
        }

        tab_bar = tab_bar
            .child(div().flex_1())
            .child(
                div()
                    .id("new-tab")
                    .px_2()
                    .on_click(cx.listener(|this, _, window, cx| this.new_tab(window, cx)))
                    .child("+"),
            )
            .child(
                div()
                    .id("split-h")
                    .px_2()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.split_active(SplitDirection::Horizontal, window, cx);
                    }))
                    .child("◫"),
            )
            .child(
                div()
                    .id("split-v")
                    .px_2()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.split_active(SplitDirection::Vertical, window, cx);
                    }))
                    .child("⬓"),
            )
            .child(
                div()
                    .id("rename-tab")
                    .px_2()
                    .on_click(cx.listener(|this, _, window, cx| {
                        if let Some(tab) = this.active_tab() {
                            let id = tab.id;
                            this.start_rename(id, window, cx);
                        }
                    }))
                    .child("✎"),
            )
            .child(
                div()
                    .id("maximize-pane")
                    .px_2()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_maximize(cx)))
                    .child("⛶"),
            );

        let content = match self.tabs.get(self.active) {
            Some(tab) if tab.maximized => tab.active_pane.clone().into_any_element(),
            Some(tab) => tab.tree.render(),
            None => div().into_any_element(),
        };

        let mut root = v_flex()
            .size_full()
            .bg(rgb(0x101014))
            .text_color(rgb(0xdddddd))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_shortcut(event, window, cx);
            }))
            .child(tab_bar);
        if self.rename_target.is_some() {
            root = root.child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(0x2a2a33))
                    .child(Input::new(&self.rename_input))
                    .child(
                        div()
                            .id("rename-ok")
                            .px_2()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.commit_rename(window, cx);
                            }))
                            .child("OK"),
                    ),
            );
        }
        root.child(div().flex_1().overflow_hidden().child(content))
    }
}
