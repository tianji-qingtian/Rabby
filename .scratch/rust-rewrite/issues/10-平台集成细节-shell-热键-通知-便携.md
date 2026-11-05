<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T07 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.236Z -->
<!-- DSH-ASSIGNEES: lead -->
# 10: 平台集成细节：shell、热键、通知、便携

Type: research

Status: resolved
## Question

目标：盘点 Tabby 在三平台上依赖的系统集成，确定 Rust 侧实现路径与风险。

要回答：
- Windows：PowerShell / PS Core / CMD / WSL / Git-Bash / Cygwin / MSYS2 / Cmder / VS 的发现与启动参数、ConPTY、Clink 集成、注册表、UAC、便携模式、Windows OpenSSH agent/Pageant；
- macOS：默认 shell、登录 shell、Touch Bar、vibrancy、notarization、全局热键、Dock/菜单栏；
- Linux：默认 shell、Wayland/X11 全局热键、托盘、包格式（deb/rpm/AppImage/snap）；
- 通用：Quake console（全局热键 + 无边框窗口）、通知（进程完成/进度）、单实例、文件拖放、URL 打开、剪贴板、系统主题跟随；
- 各能力的 crate 选项与平台限制；
- 与 GPUI 能力的交叉点（窗口层级、置顶、透明、多窗口）。

产出：`.scratch/rust-rewrite/research/10-platform-integration.md`。

## Answer

结论：平台集成独立成 rabby-platform crate，shell 发现走「注册表/dscl/getpwuid 为主的单一权威源」，Windows 只做 ConPTY，Quake/热键三平台分级（Wayland 走 ashpd GlobalShortcuts），Clink 因 GPLv3 不随包分发。
· Shell：Windows 8 类 shell 的探测源与启动参数已盘点（含 MSYS2 四环境、WSL Lxss Flags&8 判 WSL2、VS 硬编码 Community 需改 vswhere）；macOS 用 dscl+--login；Linux 用 passwd+/etc/shells。
· ConPTY：build 门 17692/18309/19041/21376，bundled conpty.dll 的意义是 VT mouse tracking 透传，失败必须降级；UAC 提权须由 helper 自建 ConPTY（句柄不跨完整性级别）。
· GPUI 0.2.2（已在 crates.io）：window_background/TitlebarOptions/FileDropEvent/ExternalPaths 可用，但缺 always_on_top、opacity、skip_taskbar、hide、tray、global hotkey、notification —— 需 WindowExt 平台扩展。
· 风险：Wayland 热键无统一后端、Linux 托盘不可靠、Clink GPLv3、Windows 通知需 AUMID、macOS 通知需 bundle+授权。
· 研究笔记：.scratch/rust-rewrite/research/10-platform-integration.md（约 2320 字 + 5 张表，全部结论带 文件:行号 或 crates.io/docs.rs 链接）。

## 进度
