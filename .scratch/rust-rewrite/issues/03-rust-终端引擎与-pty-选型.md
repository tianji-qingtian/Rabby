<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T03 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.236Z -->
<!-- DSH-ASSIGNEES: lead -->
# 03: Rust 终端引擎与 PTY 选型

Type: research

Status: resolved
## Question

目标：为 Tabby 级终端（VT220+extensions、xterm.js 6 行为、Unicode 宽字符、ligatures、图片、search/serialize、大吞吐）选一个可维护的 Rust 内核与 PTY 层。

要回答：
- 候选：wezterm-term、alacritty_terminal、vte、gpui-kit Terminal、其他（含自研）——VT 覆盖、屏幕模型/scrollback、宽字符/emoji/组合字符、ligatures、sixel/iTerm2 图片、OSC 52/8/9;4、bracketed paste、mouse、search/serialize、性能与内存；
- PTY：portable-pty / pty-process / 直接 ConPTY；Windows ConPTY、Unix openpty、resize、信号、进程树/工作目录、shell integration；
- 与 GPUI 的集成方式：网格绘制、字体度量、连字 shaping、dirty-row 重绘、输入法候选窗；
- xterm.js 6 与候选内核的行为差异清单（会破坏兼容的点）；
- 许可证、维护活跃度、被哪些产品使用。

产出：`.scratch/rust-rewrite/research/03-terminal-engine-pty.md`，含对比表与推荐。

## Answer

推荐：内核 wezterm-term（git pin + 自建 TerminalCore façade）、PTY portable-pty、GPUI 自绘网格；若 v1 砍掉图片协议与 bidi，则改选 alacritty_terminal 0.26。
① wezterm-term 是唯一默认覆盖 SIXEL/iTerm2/kitty 图片、OSC 8/52、OSC 9;4 进度、DECSET 2026、resize rewrap、grapheme Cell 的内核；代价是未发布 crates.io（只有第三方转发）、API 无 semver，须 pin/fork。
② alacritty_terminal 0.26（Apache-2.0、已发布）有行 damage、regex search、reflow，但无任何图片协议，Cell 为 char + ≤9 zero-width，emoji/CJK 是结构性短板。
③ PTY 选 portable-pty：Unix openpty+setsid+TIOCSCTTY，Windows 动态加载 ConPTY；pty-process 无 Windows，conpty crate 停更；Windows 进程树需自建 Job Object（portable-pty 只有 TerminateProcess）。
④ gpui-kit 0.9.0 无 Terminal 组件：网格用 shape_line+Quad+字形 atlas+paint_image 自绘；GPUI 只有 view 级 invalidate，dirty-row 由模型 damage 驱动重建行数据。
⑤ 兼容契约以 xterm.js 6 的 typings 为准：vtExtensions（kittyKeyboard/win32InputMode 默认关、colorSchemeQuery 开）、windowsPty 的 ConPTY reflow 补偿必须对齐。
研究笔记：.scratch/rust-rewrite/research/03-terminal-engine-pty.md

## 进度
