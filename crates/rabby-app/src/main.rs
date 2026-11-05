use gpui_kit::*;
use rabby_core::{Shell, ShellType};
use rabby_platform::{DefaultPlatform, ShellCatalog};
use rabby_pty::PtySession;
use rabby_terminal::TerminalCore;
use rabby_terminal_view::TerminalPane;
use rabby_ui::Workspace;

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

fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(960.0), px(640.0)), cx)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            let shell = DefaultPlatform::new()
                .default_shell()
                .unwrap_or_else(fallback_shell);
            let mut pty = PtySession::spawn(&shell, 80, 24).expect("spawn local PTY");
            let writer = pty.take_writer().expect("PTY writer");
            let core = TerminalCore::new(80, 24, 25_000, writer);
            let pane = cx.new(|cx| TerminalPane::new(window, cx, core, pty));
            cx.new(|_| Workspace::new(pane))
        })
        .expect("open window");
    });
}
