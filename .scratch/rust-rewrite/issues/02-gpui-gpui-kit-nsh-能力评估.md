<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T02 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.236Z -->
<!-- DSH-ASSIGNEES: lead -->
# 02: GPUI / GPUI Kit / NSH 能力评估

Type: research

Status: resolved
## Question

目标：判断 GPUI、GPUI Kit、NSH 能否承载 Tabby 的桌面 UI 与终端渲染，给出分工建议与风险清单。

要回答：
- GPUI 当前版本与平台成熟度（macOS/Windows/Linux；GPU 后端；文本 shaping/字体回退；IME/CJK；AccessKit 无障碍；剪贴板/拖放/全局快捷键/多窗口/托盘；headless 测试）；
- GPUI Kit 的组件覆盖：DockArea/TabGroup/分屏、Terminal builder/TerminalState、表格、设置表单、主题、命令面板、通知、WASM；版本、发布节奏、许可证；
- NSH 的定位与边界：GPUI backend 如何嵌入 GPUI Kit 应用；TSX/运行时模块/签名分发；许可证与成熟度；对终端这类自绘密集场景是否合适；
- 明确“GPUI Kit 能做 / 不能做 / 需要自研”的清单，以及 NSH 只在什么条件下值得引入；
- 给出 2–3 种 UI 技术组合的取舍。

产出：`.scratch/rust-rewrite/research/02-gpui-kit-nsh-capability.md`，附官方文档/源码链接与版本号。

## Answer

一句话结论：v1 用 GPUI Kit 0.9 做桌面外壳 + 自研 terminal，NSH 不进 v1 主路径。要点：① GPUI 无稳定 crate：官方 gpui 停在 0.2.2，生态实际用 gpui-pre 0.3.11（Zed main 快照、周更、发布者非 Zed 官方），Kit 用 =0.3.11 死锁，需 fork/vendor + 季度升级窗口；② 三平台平台层齐全（macOS CoreText/NSTextInputClient、Windows DirectWrite/IME、Linux X11+Wayland/cosmic-text/wgpu、accesskit 三平台、文件拖放 FileDropEvent、headless TestAppContext），但 Linux 最弱，且无托盘、无全局热键 API；③ GPUI Kit 0.9 有 Dock（DockArea/DockLayout/tabs/分屏/持久化）、Settings 表单、Theme、Command 命令面板、Notification+OS 通知、表格/虚拟列表/编辑器/WebView，但**没有 Terminal 组件**（0.9.0 docs.rs 源码树、workspace、官网 /component/terminal 均无），终端 widget 必须自研（可参照 Zed crates/terminal = alacritty_terminal + vte，Apache-2.0）；④ NSH 0.4.0 仅 3 周龄，GPUI 后端依赖 gpui-fast 0.1.9 而非 gpui-pre，与 Kit 混用需整体切 gpui-fast feature，只适合 v1.x 的扩展/AI UI 运行时；⑤ WASM 官方声明仅用于 showcase（单窗口、无字体库），v1 不做 Web。研究笔记：.scratch/rust-rewrite/research/02-gpui-kit-nsh-capability.md

## 进度
