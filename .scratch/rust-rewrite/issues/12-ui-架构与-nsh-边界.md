<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:grilling -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T12 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.237Z -->
<!-- DSH-BLOCKED-BY: 02, 08, 09 -->
<!-- DSH-ASSIGNEES: lead -->
# 12: UI 架构与 NSH 边界

Type: grilling

Blocked by: 02, 08, 09
Status: resolved
## Question

目标：锁定 GPUI Kit 与 NSH 的职责边界、应用外壳架构与主题/设置体系。

要讨论：
- 应用外壳：窗口/标签/分屏/Dock/Quake 用 GPUI Kit 的哪些组件，哪些自绘；
- 设置/命令面板/快捷键/通知/选择器/迁移向导的组件映射；
- NSH 是否引入、以什么形态（Web/移动/运行时可下发 TSX/仅组件）；引入后的状态/主题/输入一致性；
- 主题与配色：从 Tabby color scheme 到 GPUI Kit token 的映射；
- 可访问性与 IME 的验收；
- 与终端渲染的边界（谁持有焦点/滚动/选择）。

产出：UI 架构决策 + 组件映射表，写回 Answer。

## Comments

### local — 2026-11-05T03:17:10.017Z
草案 v0.1 已落盘：.scratch/rust-rewrite/specs/ui-architecture.md。含分层/所有权、组件映射、主题、无障碍/IME、焦点/滚动/选区、会话恢复、NSH 边界；待拍板 §0 的 7 项。

## Answer

UI 架构：自研 Workspace/PaneTree/TabStrip（Kit 只做外观零件）；v1 顶部标签 + 四向嵌套分屏 + 最大化，四边标签/Quake/多窗口放 v1.x；TerminalPane 拥有 TerminalCore/PTY/焦点/选区/滚动；Kit 做 Settings/Command/通知/菜单，终端与 profile/迁移逻辑自研；Kit token 外壳 + 独立 TerminalPalette，不做 CSS 主题；v1 无障碍/IME 冒烟 + 三平台手工矩阵；NSH 不进 v1，v1.x 只评估扩展/运行时 UI。详见 .scratch/rust-rewrite/specs/ui-architecture.md

## 进度

- 进度：40%（2026-11-05T11:17:10+08:00）：UI 架构草案已写；等用户拍板 §0 的 7 项。
