<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T01 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.235Z -->
<!-- DSH-ASSIGNEES: lead -->
# 01: Tabby 功能与插件面盘点

Type: research

Status: resolved
## Question

目标：把 Tabby 当前桌面版的功能面、用户可见行为、插件 API 面盘点清楚，作为 v1 边界与迁移决策的事实底稿。

要回答：
- 桌面功能清单（终端、SSH/SFTP、串口、Telnet、Zmodem、分屏/标签、Quake、会话恢复、配色/主题、快捷键、i18n、便携、更新、遥测等），每项给出用户可见行为与关键配置项；
- 插件机制：Angular NgModule 扩展点清单（toolbar / tab context menu / profile provider / terminal middleware / hotkey / color scheme 等）、分发方式（npm keyword + plugin manager）、内置插件与社区插件依赖的 API 面；
- 哪些是独立产品能力、哪些是平台适配、哪些属于插件生态；
- 输出 parity matrix（功能 × 优先级 × 实现复杂度 × 是否可延后），标注“不看源码会漏掉”的隐性行为（progress detection、bracketed paste、ligatures、unicode11、image addon、Clink、WSL、portable mode 等）。

产出：`.scratch/rust-rewrite/research/01-tabby-feature-plugin-audit.md`，每条结论带源码路径/行号或官方文档链接；不要复制大段代码。

## Answer

Tabby 的功能面不是重写的难点，隐性行为和插件生态才是；SSH 层上游已用 Rust（russh）实现，可直接复用。
1) SSH 全栈（认证/channel/SFTP/X11/jump host/代理/Pageant）已由 russh 承载（tabby-ssh/src/session/ssh.ts:21、app/package.json:35），Rabby 应直接依赖 russh crate，删掉 N-API 层。
2) 重写主体是 VT 内核 + UI 框架：核心包约 26.4k 行 TS（core 8830 / terminal 5156 / ssh 3617 / electron 3374）。
3) 插件 API 面 = 31 个 Angular abstract class + DI multi provider，扩展单元是 NgModule；无跨语言 ABI，兼容层等于内嵌 Electron，确认不可兼容（与 map 的 Out of scope 一致），只做能力对齐。
4) 真正会"露馅"的是 18 条隐性行为：progress detection（正则 + 非 alt-screen + >0%）、8MB 预缓冲、bracketed paste/多行确认、copyAsHTML 默认开、Unicode11 宽度、sixel、ligatures（Web 禁用）、OSC cwd 上报、PTY re-attach 恢复、Windows 注册表环境重建、ConPTY 双路径、Clink、WSL 注册表枚举、便携模式等。
5) v1 边界建议维持地图假设，但把 18 条写成验收用例；串口/Telnet/Zmodem 可延后（最薄的两个包共 1211 行）；PTY re-attach 建议降 P1，遥测建议默认关。
研究笔记：.scratch/rust-rewrite/research/01-tabby-feature-plugin-audit.md（含 parity matrix、扩展点清单、隐性行为表、风险表）

## 进度
