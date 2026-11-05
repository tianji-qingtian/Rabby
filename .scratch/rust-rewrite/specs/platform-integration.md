# Rabby 平台集成与系统能力（草案 v0.1）

> 状态：15 票「平台集成与系统能力」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A，本文作为 v1 平台基线。
> 输入：10（平台集成研究）、07（v1 范围）、08（平台/分发）、12（UI 架构）、13（配置根/密钥）；GPUI/GPUI Kit 的窗口能力限制。
> 一句话：v1 做 shell 目录、PTY/ConPTY、单实例、URL scheme、拖放、通知（默认关）、主题跟随、剪贴板、配置根；Quake/托盘/全局热键/便携/UAC 放 v1.x；平台差异收进 rabby-platform 的 trait。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A rabby-platform crate + trait；Q2=A shell 每类权威源 + ProbeResult + TTL；Q3=A Windows 系统 ConPTY、不 bundle dll/winpty、UAC v1.x、Clink 不随包；Q4=A v1 shell/PTY/单实例/URL/拖放/通知默认关/主题/剪贴板/配置根，Quake/托盘/全局热键/完整便携/UAC v1.x；Q5=A macOS dscl+--login+locale、Linux getpwuid_r+/etc/shells、Wayland ashpd/X11 global-hotkey、失败可见；Q6=A 通知默认关、URL 白名单、拖放按 shellType 引号；Q7=A macOS 先行、Windows/Linux 同 CI。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | 平台抽象层 | A. 独立 rabby-platform crate：ShellCatalog、PtyBackend/ConPTY 辅助、HotkeyBackend、Notifier、SingleInstance、WindowExt、Clipboard、UrlOpener、SystemTheme；cfg 平台模块 + 测试替身 | B. 每个平台一个 crate：边界更硬，但共享语义/测试成本高；C. 直接写在 AppShell：平台差异污染主循环 |
| Q2 | Shell 发现 | A. 每类 shell 一个权威源（Win 注册表/App Paths/Lxss/vswhere；macOS dscl；Linux getpwuid_r + /etc/shells）+ ProbeResult(skipped reasons) + TTL 缓存 + 设置页刷新；App Execution Alias 特殊处理 | B. PATH/which 探测：漏 PowerShell 7/WSL/VS；C. 最小几种 shell：日用不足 |
| Q3 | Windows PTY/提权/Clink | A. 只用系统 ConPTY（build>=17692，按 19041/21376 分支）；v1 不 bundle conpty.dll、不做 winpty、不做 UAC 提权；Clink 只探测已装、不随包分发（GPLv3） | B. v1 bundle conpty.dll / 做 UAC helper：鼠标透传/管理员会话更好，但版本/许可/窗口句柄风险高；C. winpty 回退：无必要 |
| Q4 | v1 vs v1.x 系统能力 | A. v1：shell 目录、PTY/ConPTY、单实例、URL scheme、拖放、通知默认关、主题跟随、剪贴板、配置根、flash frame；Quake、托盘、全局热键、完整便携模式、Jump List/Dock 菜单/Touch Bar、UAC helper 放 v1.x | B. Quake/托盘/全局热键也进 v1：GPUI 没有这些 API，平台扩展风险最高；C. 系统集成全放 v1.x：v1 体验不完整 |
| Q5 | macOS/Linux 细节 | A. macOS dscl + --login + locale 补齐；Linux getpwuid_r + /etc/shells + 设置里显式默认 shell；Wayland 全局热键 ashpd portal、X11 global-hotkey；失败可见 | B. 照抄 Tabby 的 LOGNAME/passwd 与静默失败：兼容但脆弱；C. 只做默认 shell：不够 |
| Q6 | 通知/URL/拖放安全 | A. 通知默认关、逐标签开启；notify-rust + Windows AUMID / macOS bundle id / Linux D-Bus；URL 只开 http/https 白名单；拖放按 shellType 做 unix/PowerShell/cmd 引号；B. 通知默认开；C. 不做通知/拖放 |
| Q7 | 测试矩阵/发布 | A. macOS 先行，Windows/Linux 同 CI；v1 P0 三平台都过；shell 矩阵、PTY resize、ConPTY、单实例、URL、拖放、通知；热键/托盘 Wayland 手工矩阵；B. v1 只保证 macOS；C. 所有平台所有能力 v1 |

## 1. rabby-platform 抽象

    trait ShellCatalog {
        fn discover(&self) -> ProbeResult<Vec<Shell>>;
        fn refresh(&self);
    }

    trait SingleInstance {
        fn acquire(&self, argv: Vec<String>) -> Result<()>;
        fn on_second_instance(&self, cb: ...);
    }

    trait Notifier { fn notify(&self, notification: Notification) -> Result<()>; }
    trait Clipboard { fn read_text(&self) -> Result<String>; fn write_text(&self, text: &str) -> Result<()>; }
    trait UrlOpener { fn open(&self, url: &str) -> Result<()>; }
    trait SystemTheme { fn appearance(&self) -> Appearance; }
    trait WindowExt {
        fn set_always_on_top(&self, window: &Window, on: bool);
        fn set_opacity(&self, window: &Window, opacity: f32);
        fn set_skip_taskbar(&self, window: &Window, skip: bool);
        fn set_visible_on_all_workspaces(&self, window: &Window, on: bool);
        fn flash_frame(&self, window: &Window);
    }

- 平台模块：macos、windows、linux；每项有 no-op/测试替身，单测不依赖真实系统。
- GPUI 有能力的不重复封装（主题用 WindowAppearance、拖放用 FileDropEvent）；没有的走原生。
- WindowExt 的句柄来源（raw-window-handle 是否拿到真实 HWND/NSWindow）是 15 的 spike 项；拿不到就降级为 v1.x。
- 配置根、keychain、更新、签名分别归 13/08，不在 15 重复。

## 2. Shell 目录

- Shell 字段：id、name、command、args、env、cwd、homeDirArgs、fsBase、shellType（unix/powershell/cmd/cygwin）、icon、hidden。
- 权威源：
  - Windows：PowerShell 5（System32 路径）、PowerShell Core（App Paths → 三条 well-known → PATH；App Execution Alias 用父目录列举绕过 stat 失败）、CMD、Clink（仅探测已装）、WSL（HKCU Lxss，Flags&8 判 WSL2，build>=19041 才用 --cd）、Git Bash（GitForWindows InstallPath）、Cygwin（setup rootdir）、MSYS2（msys64 目录 + env 枚举）、Cmder（CMDER_ROOT）、VS DevCmd（vswhere 定位，不硬编码 Community）。
  - macOS：dscl UserShell + --login；/etc/shells 列表；locale 只在没有 LC_ALL 时注入。
  - Linux：getpwuid_r 取登录 shell；/etc/shells 列表；设置里显式选默认 shell；不要用 LOGNAME 匹配 passwd。
- ProbeResult：shells + skipped Vec(id, reason)；设置页可展开；启动 TTL + 刷新按钮；装完 WSL/Git 不用重启。
- TERM/identification：wt/cygwin/默认三档注入 WT_SESSION=0 或 TERM=cygwin；Clink/Cygwin 依赖这个。
- 拖放/命令引用：按 shellType 三分支（unix '...'、PowerShell 单引号幂双写、cmd ^! / "" / %%）。
- 默认 shell：provider 顺序推导，不硬编码；用户可覆盖；找不到时 fallback /bin/sh。

## 3. PTY / ConPTY / 提权 / Clink

- PTY：portable-pty 0.9；macOS/Linux openpty + setsid/TIOCSCTTY；Windows 动态加载 ConPTY。
- Windows：只用系统 ConPTY；build>=17692；reflow 按 21376 分支；v1 不 bundle conpty.dll、不做 winpty；保留 conpty_dll 开关位；spawn 失败要可诊断。
- 进程树：Windows 自建 Job Object（portable-pty 只有 TerminateProcess）；Unix 用进程组/信号；退出行为：
  - pauseAfterExit：非用户主动关闭时打印 Press any key to close，不闪退。
  - 工作目录：OSC 7/133 优先 + 进程 CWD 查询兜底 + 输出流猜测只作最后一级；Windows 查询有权限/时序问题。
- UAC：v1 不做；v1.x 独立提权 helper 内建 ConPTY + 控制管道（照 Tabby UAC.exe），因为 ConPTY 句柄不能跨完整性级别。
- Clink：GPLv3，不随包分发；探测已安装；可提示用户自行安装；不静默注入。
- 环境：Windows 注册表重建 HKLM/HKCU/Volatile，Path 追加不是覆盖；macOS locale 补齐；Linux 继承。

## 4. 系统能力清单

| 能力 | v1 | v1.x | 说明 |
|---|---|---|---|
| Shell 目录/刷新 | P0 | — | ProbeResult + TTL |
| PTY/ConPTY/resize | P0 | — | 09 已验证 macOS；Windows CI 待补 |
| 单实例/argv 转发 | P0 | — | Unix socket / Windows named pipe；interprocess |
| URL scheme rabby:// | P0 | — | 白名单 command + 类型化参数；macOS open-url / Windows 注册表 / Linux .desktop |
| 文件拖放 | P0 | — | GPUI FileDropEvent + ExternalPaths；按 shellType 引号 |
| 通知 | P0（默认关） | — | notify-rust；Windows AUMID、macOS bundle id、Linux D-Bus；逐标签开启 |
| 主题跟随 | P0 | — | GPUI WindowAppearance；不用 dark-light |
| 剪贴板 | P0（text） | HTML | arboard；copyAsHTML P1 |
| 配置根/便携检测 | P0 | 完整便携 | 13 的优先级；exe 同目录 data/ |
| flash frame | P0 | — | WindowExt；任务栏闪烁 |
| 全局热键 | 不做 | P1 | global-hotkey（Win/macOS/Linux X11）+ ashpd（Wayland）；失败可见 |
| Quake | 不做 | P1 | 置顶/全工作区/跳过任务栏/透明度/隐藏；GPUI 无 hide，WindowExt 原生 |
| 托盘 | 不做 | P1 | tray-icon/ksni；Linux Wayland 不可靠；不默认关窗到托盘 |
| Jump List/Dock 菜单/Touch Bar | 不做 | P1 | 平台增强；Touch Bar 建议不做 |
| UAC helper | 不做 | P1 | 提权 ConPTY helper |
| 自动更新 | 不做 | v1.x/08 | 08 决策 |

## 5. 全局热键 / Quake / 托盘（v1.x 设计）

- 全局热键：Windows RegisterHotKey（同线程消息循环）、macOS Carbon RegisterEventHotKey（主线程）、Linux X11 XGrabKey（global-hotkey）、Wayland ashpd GlobalShortcuts portal；注册失败/被占用/无后端要在设置页显示未生效原因，不能静默。
- Quake：WindowExt 原生：
  - always-on-top：Win SetWindowPos(HWND_TOPMOST)、macOS NSWindow.level、Linux EWMH/层壳。
  - visible-on-all-workspaces：macOS collectionBehavior、Windows 虚拟桌面、Linux EWMH。
  - skip-taskbar/opacity/hide：GPUI 没有，走原生；隐藏时保留 view 状态 vs 销毁重建要 spike。
  - 失焦隐藏/热区贴边：v1.x 评估；触屏/多显示器要测。
- 托盘：tray-icon/ksni；macOS NSStatusItem、Windows Shell_NotifyIcon、Linux StatusNotifierItem/AppIndicator；Wayland/GNOME 不可靠时不默认启用；单实例二次启动可唤起窗口作为兜底。

## 6. 通知 / URL / 拖放 / 单实例

- 通知：默认关；进程完成/标签活动逐标签开启；应用内 toast + 系统通知；Windows 需 AUMID（安装器写快捷方式；便携降级应用内）；macOS 需 bundle id + 用户授权；Linux D-Bus；通知内容不含终端输出/密钥。
- URL：只接受 http/https；rabby:// 的 command 白名单 + 类型化参数；其他 scheme 弹确认。
- 拖放：取真实路径（GPUI ExternalPaths）；按 shellType 做引号；不把路径拼进 shell 字符串以外的上下文。
- 单实例：第二个实例把 argv/cwd 转给第一个；macOS open-url 走同一入口；失败提示。

## 7. 测试矩阵

- CI：macOS arm64、Windows x64、Ubuntu 22.04（X11；Wayland 手工）。
- Shell 矩阵：PowerShell 5/7、CMD、Clink 已装/未装、WSL1/2、Git Bash、Cygwin、MSYS2、Cmder、VS DevCmd（vswhere）、macOS 默认+/etc/shells、Linux 默认+/etc/shells。
- PTY：resize/SIGWINCH（09 已过 macOS）、ConPTY build 分支、pauseAfterExit、cwd（OSC/进程查询/猜测）。
- 系统：单实例 argv、URL scheme、拖放引号、通知权限/AUMID、主题跟随、剪贴板。
- v1.x 手工：全局热键 X11/Wayland/macOS Secure Input、Quake 多显示器/全屏空间、托盘 Wayland/GNOME。
- 性能：shell 发现 TTL、PTY spawn 到首字节、flash frame 开销。

## 8. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| GPUI 没有 hide/置顶/托盘/热键 | Quake/托盘做不了 | v1.x WindowExt 原生；先 spike 句柄获取 |
| Wayland 热键/托盘无统一后端 | Linux 功能缺失且静默 | 失败可见；文档说明；单实例兜底 |
| ConPTY 跨提权不可行 | 管理员会话缺失 | v1 不做；v1.x helper |
| Clink GPLv3 | 许可风险 | 不 bundle；探测已装 |
| App Execution Alias/WSL 注册表 | shell 漏检 | 父目录列举绕过；ProbeResult 显示原因 |
| Windows 通知 AUMID/便携 | 通知不显示 | 安装器写 AUMID；便携降级应用内 |
| macOS 公证/隐私串 | PTY 子进程被 TCC 拦 | 08 的 entitlements + 隐私串；原型期验证 |
| 平台差异污染主循环 | 维护成本 | rabby-platform trait + cfg；单测替身 |

## 9. ADR 草稿要点

- ADR-0011：rabby-platform crate + trait；shell 发现权威源 + ProbeResult + TTL。
- ADR-0012：Windows 只用系统 ConPTY；不 bundle dll/不做 winpty；UAC v1.x；Clink 不随包。
- ADR-0013：v1 系统能力清单；Quake/托盘/全局热键/完整便携/UAC v1.x；失败可见。

## 10. 证据

- 10：.scratch/rust-rewrite/research/10-platform-integration.md
- 07：.scratch/rust-rewrite/specs/v1-scope.md
- 08：.scratch/rust-rewrite/specs/platform-distribution.md
- 12：.scratch/rust-rewrite/specs/ui-architecture.md
- 13：.scratch/rust-rewrite/specs/config-secrets-migration.md
