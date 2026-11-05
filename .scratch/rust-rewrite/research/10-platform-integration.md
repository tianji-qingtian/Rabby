# 10 平台集成细节：shell、热键、通知、便携

- 票号：10（wayfinder:research），地图：`rabby：用 Rust 重写 Tabby`
- 上游基准：Eugeny/tabby @ `467ce8e`（2026-10-06），源码在 `/tmp/tabby-recon`
- 取证日：2026-11-05；Rust crate 版本均取自当日 crates.io / docs.rs
- 方法：只读一手源码 + 官方文档；Tabby 行号均指上述 commit。未标注来源的句子是我的判断，已在文中写明"判断"。

## 0. 结论（先看这段）

1. **平台集成独立成 `rabby-platform` crate，对上层只暴露 trait**（`ShellCatalog`、`HotkeyBackend`、`Notifier`、`SingleInstance`、`WindowExt`）。理由：这批能力在三平台差异最大、又最容易污染主循环；Tabby 把它们散在 `tabby-electron/src/services/*` 与 Angular 服务里，是它插件层耦合最重的地方（判断）。
2. **Shell 发现按"平台原生数据源优先、PATH 探测兜底"实现**，不要先跑 `which`。Windows 用注册表（`App Paths\pwsh.exe`、`GitForWindows`、`Cygwin\setup`、`Lxss`），macOS 用 `dscl` 读 `UserShell`，Linux 用 `getpwuid_r` + `/etc/shells`。见 1.1。
3. **Windows 只做 ConPTY，不做 winpty**，并按 build 号分级降级：>=`19041` 可用 bundled `conpty.dll`（Tabby 用它透传 VT mouse tracking），否则用系统 ConPTY；bundled 失败必须回退而不是报错（`tabby-local/src/session.ts:118-133`）。v1 建议先不 bundle dll，只留降级路径。
4. **UAC 提权 v1 不做**，或做成"提权 helper 自己创建 ConPTY"的独立二进制。原因：ConPTY 句柄无法跨完整性级别传递，Tabby 的 `UAC.exe` 正是把 `CreatePseudoConsole` 搬进提权进程再走控制管道（`tabby-uac/UAC/UAC.cpp:36-70`）。
5. **全局热键/Quake 三平台分级**：Windows/macOS 直接做（RegisterHotKey / Carbon RegisterEventHotKey）；Linux 先只保证 X11，Wayland 走 `ashpd` 的 GlobalShortcuts portal，检测不到就显式降级并提示，不静默失败。
6. **Clink 不随包分发**。Tabby 内置 GPLv3 的 clink（`extras/clink/LICENSE` 首行即 GPLv3），与 MIT 主程序同包分发有许可风险；改为探测已安装 Clink 后注入，或做成可选下载。
7. **通知默认关**，沿用 Tabby 的"按标签手动开启"语义（`tabby-core/src/tabContextMenu.ts:204-250`）；通知走 `notify-rust`，Windows 必须设 AppUserModelID，macOS 必须带 bundle id。
8. **便携模式 = 可执行文件同目录 `data/` 即数据根**，直接兼容 Tabby 语义（`app/lib/portable.ts:5-11`），配置根优先级见 5.3。

## 1. Tabby 的一手事实

### 1.1 Shell 发现与启动矩阵

| 平台 | Shell | 探测方式 | 启动 | 关键环境/备注 | 出处 |
|---|---|---|---|---|---|
| Win | PowerShell 5 | `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe`，再 `which` | `-nologo` | 有 `WT_SESSION=0`/`TERM=cygwin` 的"终端识别"映射 | `shells/windowsStock.ts:65-79`、`windowsBase.ts:16-25` |
| Win | PowerShell Core | `App Paths\pwsh.exe`（HKLM→HKCU）→ 三条 well-known 路径 → PATH | `-nologo` | 注释显式指出 App Execution Alias 会让 `fs.stat` 失败，需退回目录列举 | `shells/powershellCore.ts:29-50`、`windowsBase.ts:39-55` |
| Win | CMD (stock) | `cmd.exe` | 无参 | — | `shells/windowsStock.ts:55-62` |
| Win | CMD (clink) | 包内 `resources/extras/clink/clink_<arch>.exe` | `cmd.exe /k <clink> inject` | 置 `WT_SESSION=0` 让 Clink 不模拟 ANSI | `shells/windowsStock.ts:25-54` |
| Win | WSL | `HKCU\...\Lxss`，`Flags & 8` 判 WSL2（`\\wsl$\<distro>`），否则 `BasePath\rootfs` | `wsl.exe` / `-d <name>` | `--cd ~` 需 build>=19041；`TERM=xterm-color`+`COLORTERM=truecolor` | `shells/wsl.ts:54-136`、`utils.ts:5-11` |
| Win | Git Bash | `Software\GitForWindows`（HKLM→HKCU）`InstallPath` | `bin/bash.exe --login -i` | — | `shells/gitBash.ts:30-48` |
| Win | Cygwin 32/64 | `Cygwin\setup` / `WOW6432Node\Cygwin\setup` 的 `rootdir` | `bin/bash.exe --login -i` | `TERM=cygwin` | `shells/cygwin32.ts:27-43`、`cygwin64.ts:27-43` |
| Win | MSYS2 ×4 | `%SystemRoot%/../msys64` 存在性 | `msys2_shell.cmd -defterm -here -no-start -<env>` | 环境枚举 `msys/mingw64/clang64/ucrt64` | `shells/msys2.ts:22-47` |
| Win | Cmder / Cmder PS | 仅环境变量 `CMDER_ROOT` | `cmd.exe /k vendor/init.bat`；PS 走 `-command Invoke-Expression profile.ps1` | `TERM=cygwin` | `shells/cmder.ts:21-57` |
| Win | VS 开发者命令行 | 遍历 `Microsoft Visual Studio\<ver>\**Community**\Common7\Tools\VsDevCmd.bat` | `cmd.exe /k <bat>` | 硬编码 Community，Pro/Enterprise/BuildTools 漏检 | `shells/vs.ts:30-58` |
| macOS | OS 默认 | 主进程 `dscl . -read /Users/$LOGNAME UserShell`，失败回落 `/bin/bash` | `--login` | 结果在渲染侧缓存 | `app/lib/app.ts:63-69`、`shells/macDefault.ts:23-44` |
| macOS | `/etc/shells` 全部 | 读文件（Solus 走 `/usr/share/defaults/etc/shells`） | `-l` | id 用 slugify | `shells/posix.ts:17-41` |
| Linux | 用户默认 | 用 `LOGNAME` 在 `/etc/passwd` 找行，取第 7 字段；失败回落 `/bin/sh` | `--login` | 找不到时打 warn | `shells/linuxDefault.ts:21-47` |
| Linux | `/etc/shells` 全部 | 同上 | `-l` | — | `shells/posix.ts:17-41` |

"OS default" 的挑选逻辑很朴素：按 PowerShell Core → WSL → stock 的顺序取第一个非空列表的最后一个元素（`shells/winDefault.ts:34-49`）。这套"provider 返回列表、默认项改写 id=default 且 hidden"的模型值得照抄——它让"默认 shell"变成推导结果而非硬编码。

**这张表最值得学的是"探测顺序"，也是最值得改的是"错误处理"**。Tabby 的每条探测都容忍失败（`try/catch` 返回空数组），因此某个注册表项读不到时用户只看到 shell 少了一个，没有任何提示；排查只能靠日志。Rabby 建议给每个 provider 加 `ProbeResult { shells, skipped: Vec<(id, reason)> }`，把"没检测到 pwsh 是因为没装、还是因为注册表被策略锁了"区分开，并在设置页的"Shell 目录"里可展开查看。Windows 的 App Execution Alias（`...\WindowsApps\pwsh.exe`）是必须处理的例子：它是 reparse point，`stat` 会失败但程序可执行，Tabby 用"列举父目录再比名字"绕过去（`windowsBase.ts:39-55`），Rust 侧要复刻同样的绕过，否则 PowerShell 7 会漏检。

第二处要改的是"重扫时机"。Tabby 在应用启动时一次性构建 shell 列表，用户装完 WSL 发行版或 Git 之后必须重启。Rabby 应把探测做成可失效的缓存（设置页"刷新"按钮 + 首次打开新建标签时的 TTL 检查），成本很低但感知差异明显（判断）。

### 1.2 PTY 与 ConPTY

`tabby-local/src/session.ts:111-133` 是全仓最有信息量的一段：`name:'xterm-256color'`、`encoding:null`、`cwd` 先 `realpath` 再校验、`useConptyDll` 由 `useConPTY && build>=19041` 决定，且 spawn 失败时若开着 bundled conpty 就**再 spawn 一次关掉它**。注释写明原因：node-pty 的 bundled ConPTY 能把 VT mouse tracking 透传给全屏 TUI，系统自带的不行。

版本门依次为 17692（ConPTY 可用）、18309（stable）、19041（可用 bundled dll）、21376（reflow），见 `tabby-core/src/utils.ts:5-11`；`isWindowsBuild` 用 `os.release()` 第三段判（`app/lib/windows.ts:3-10`）。

Windows 工作目录是个坑：`session.ts:21,260-272` 用正则从**输出流**里猜 `X:\...`，再 `isDirectory` 校验，注释说明该正则会误命中命令行里的文件路径。产品化路径应是 OSC 7/633 软集成 + 进程 CWD 查询兜底（`pty.ts:140-142` 用 `getWorkingDirectoryFromPID`），猜测只作最后一级。这里的取舍是明确的：进程 CWD 查询在 Windows 上有权限/时序问题（shell 启动后 2 秒才稳定、见 `pty.ts:42-61` 的"单子进程下钻"），而 OSC 7 需要用户 shell 配合；三者叠加才是可用状态，单独任何一种都会出现"新标签在错误目录打开"的投诉。

另有一个产品级细节：`pauseAfterExit`（`session.ts:161-177`）——进程退出后若不是用户主动关闭，Tabby 打印 `Press any key to close` 并保留窗口。这对"双击 exe 一闪而过"的场景是刚需，Rust 侧不要省。

### 1.3 打包、签名与公证（与票 08 的边界）

本票只记录"平台集成会反过来约束打包"的部分，分发策略本身留给票 08。

macOS 侧的硬约束是 **Hardened Runtime + entitlements + 公证**：Tabby 在 `electron-builder.yml:61-79` 同时打开 `hardenedRuntime`、指定 `entitlements(.plist)` 与 `entitlementsInherit`，并塞进一长串 `NS*UsageDescription`（相机/麦克风/桌面/文稿/下载/网络卷/可移除卷/AppleEvents）。原因是它的子进程（PTY 里跑的任意程序）会触发这些 TCC 弹窗，而描述串缺失时 macOS 会**直接拒绝**而不是弹框。Rust 重写后这条不会消失：只要 Rabby 启动用户 shell，就必须预先声明这些用途，否则 `ls ~/Desktop` 之类的操作会被系统静默拦掉。

Windows 侧对应的是签名与时间戳（`signtoolOptions.rfc3161TimeStampServer`，并显式排除微软已签的 `OpenConsole.exe`），以及安装器的 AUMID/快捷方式——后者直接决定通知能否归组（见 4）。Linux 侧则是依赖声明：deb/rpm 里列 `gnome-keyring`、`libnotify4`、`libsecret-1-0`、`libxtst6`、`libnss3`，snap 额外要 `system-files`/`system-observe` 插槽（`electron-builder.yml:96-118`）。对 Rust 版来说 libsecret 仍然需要（`keyring` crate 的 Secret Service 后端），其余可裁。

### 1.4 平台服务实现点

| 能力 | Tabby 实现 | 出处 |
|---|---|---|
| 单实例 | `requestSingleInstanceLock`，失败即退出；`second-instance` 转发 argv+cwd 给 CLI handler | `app/lib/index.ts:83-90`、`services/hostApp.service.ts:29-46` |
| URL scheme | `setAsDefaultProtocolClient('tabby')`（dev 下带 execPath）；macOS `open-url` 事件；`tabby://cmd?k=v` 解析成 CLI argv | `app/lib/index.ts:38-45,71-81`、`app/lib/urlHandler.ts:8-57` |
| 全局热键 | 渲染侧下发 spec 列表，主进程 `globalShortcut.unregisterAll()+register`，事件 100ms 节流后 toggle 显隐 | `app/lib/app.ts:41-53,145-162` |
| Wayland 门控 | 仅当 `linux && wayland && plasma` 时要求 Plasma>=6.6，否则直接注册 | `app/lib/app.ts:229-282` |
| Quake/Dock | `dock: left/right/top/bottom` + fill/space 计算 bounds + `setAlwaysOnTop` + 失焦隐藏 | `services/docking.service.ts:26-76`、`app/lib/window.ts:116-125` |
| 置顶层级 | `setAlwaysOnTop(true,'screen-saver',1)` + `setVisibleOnAllWorkspaces(true,{visibleOnFullScreen:true})` | `app/lib/window.ts:306-332` |
| 托盘 | 仅 Windows/macOS；仅窗口不可见时创建；Linux 直接跳过 | `app/lib/app.ts:191-223` |
| 菜单栏/Dock | macOS 手搓 Application/Edit/View/Window/Help 模板；`dock.setMenu` 出 profile 列表 | `app/lib/app.ts:298-381`、`services/dockMenu.service.ts:53-68` |
| Jump List | Windows 用 `app.setJumpList` 生成 recent/profile 任务，argv 即 CLI 命令 | `services/dockMenu.service.ts:26-52` |
| 通知 | 渲染侧 Web `Notification`（进程完成 / 标签活动，逐标签开关）+ 应用内 toastr | `tabby-core/src/tabContextMenu.ts:204-250`、`services/notifications.service.ts:10-22` |
| 剪贴板 | Electron `clipboard`（text+html） | `api/platform.ts:142-160`、`services/platform.service.ts:79-86` |
| 主题跟随 | `nativeTheme.shouldUseDarkColors` → `'dark'/'light'` | `services/platform.service.ts:368-375` |
| 拖放 | `webUtils.getPathForFile` 取真实路径，再按 shellType 分别做 unix/PowerShell/cmd 引号 | `tabby-electron/src/pathDrop.ts:14-73` |
| 外链 | 白名单 scheme 直接开，其余弹确认框 | `services/platform.service.ts:157-196` |
| 提权 | 独立 `UAC.exe`（manifest `requireAdministrator`），自身持 ConPTY，控制管道传窗口尺寸/kill | `services/uac.service.ts:17-38`、`tabby-uac/UAC/app.manifest`、`UAC.cpp:36-70` |
| 便携 | exe 同目录存在 `data/` 就把 `userData` 指过去 | `app/lib/portable.ts:5-11` |
| 更新 | Linux 或便携版禁用 electron-updater，退回 GitHub Releases 跳转 | `services/updater.service.ts:25-28,101-120` |
| 公证/签名 | `hardenedRuntime` + entitlements + 一整套 `extendInfo` 隐私串；Windows `signtoolOptions` | `electron-builder.yml:47-80` |
| Linux 包 | deb（gnome-keyring/libnotify4/libsecret/libxtst6/libnss3）、rpm、pacman、snap `system-files/system-observe`、AppImage | `electron-builder.yml:81-118` |
| SSH agent | Windows：先探 `\\.\pipe\openssh-ssh-agent`，再 `isPageantRunning()`；其他平台 `SSH_AUTH_SOCK`/自定义路径 | `tabby-ssh/src/session/ssh.ts:301-330` |

### 1.5 两个容易被忽略的兼容细节

- **终端识别环境变量**：`terminal.identification` 三档（`wt`/默认/cygwin），分别注入 `WT_SESSION=0` 或 `TERM=cygwin`（`shells/windowsBase.ts:16-25`）。这不是装饰，Clink 与 Cygwin 系程序会据此切换行编辑/ANSI 行为，Rabby 必须保留同等开关。
- **macOS locale 补齐**：只有当外层没有 `LC_ALL` 时才注入 `LANG/LC_*/LC_MESSAGES`（`session.ts:89-99`）。这是"GUI 启动的进程拿不到登录 shell 的 locale"的经典修复，Rust 侧同样会遇到。

## 2. Rust 侧：crate 选项与平台限制

| 能力 | 建议 crate（版本/日期） | 平台 | 限制与坑 |
|---|---|---|---|
| 全局热键 | `global-hotkey` 0.9.0（2026-10-16） | Win/macOS/**Linux 仅 X11** | 官方 README：Windows 需同线程 win32 消息循环，macOS 需主线程事件循环；Linux 无 Wayland 后端 |
| Wayland 热键 | `ashpd` 0.13.13（含 GlobalShortcuts） | Linux | 需 portal 实现；interface 现为 version 2（CreateSession→BindShortcuts）；GNOME 侧支持需 spike |
| 通知 | `notify-rust` 4.18.2（2026-10-07） | Linux/BSD/macOS/Win | Linux/BSD 走 D-Bus（zbus）；macOS 用 `mac-notification-sys`（旧 NSUserNotification），UN 后端是 preview feature；Windows 依赖 `tauri-winrt-notification` |
| 托盘 | `tray-icon` 0.26.1（AppIndicator/KSNI）或 `ksni` 0.3.7 | Win/macOS/Linux+BSD | 需事件循环同线程；Linux 依赖 AppIndicator 或 KSNI，Wayland 下无保证 |
| 剪贴板 | `arboard` 3.7.0 | 全平台 | Linux 同时带 x11rb 与 wl-clipboard-rs（X11/Wayland 都有），Wayland 需 wl-clipboard 后端可用 |
| 系统主题 | `dark-light` 3.0.0 | 全平台 | Linux 读 XDG portal 的 color-scheme，Flatpak 内也可用；若走 GPUI 自有 `WindowAppearance` 可省 |
| 毛玻璃 | `window-vibrancy` 0.8.1 | macOS/Win | **Linux 明确不支持**（由合成器决定） |
| 打开外链/目录 | `open` 5.4.5 | 全平台 | 无 scheme 白名单，需自己加（对照 `platform.service.ts:157-196`） |
| 单实例 | `interprocess` 2.4.7（named pipe/unix socket） | 全平台 | `single-instance` 0.3.3 停更于 2021（依赖 winapi 0.3/nix 0.23），不建议 |
| 注册表 | `windows-registry` 0.100.0（微软官方，2026-09-03）或 `winreg` 0.56.0 | Windows | 只需读时 `windows-registry` 更贴合官方维护节奏 |
| 默认 shell | `libc::getpwuid_r` / `nix` | Unix | 不要用 `LOGNAME` 匹配 `/etc/passwd`（Tabby 做法，`LOGNAME` 缺失即失败） |
| 密钥/凭据 | `keyring` 4.2.0 | 全平台 | Linux 依赖 Secret Service（deb 已列 libsecret）；无后端时需降级策略 |
| PTY | 见票 03 | — | `portable-pty` 0.9.0 上次更新 2025-02-11，ConPTY 细节需自行验证 |

### 2.1 三平台各自的真难点

**Windows 的难点不是 API，是"多套真值来源"**。同一个 PowerShell 可能有 App Paths 注册项、`WindowsApps` 别名、`Program Files\PowerShell\7` 三处落点；同一个 WSL 发行版可能只有 `Lxss` 注册表项没有 `bash.exe`；Clink 有没有装、装在哪，Tabby 干脆不看系统、只认自己包内那份。Rust 侧的正确做法是**每类 shell 只认一个权威源**（powerShell = App Paths → PATH；WSL = Lxss；Git = `GitForWindows`），其余路径只做"能启动"的兜底，避免出现两个同名项。另外 ConPTY 的版本门要用 `RtlGetVersion`（或 `os-version` 0.2.1 这类封装）取真实 build（`os.release()` 在开启兼容层时会撒谎，这是 Tabby 那套 `isWindowsBuild` 的已知弱点，判断）。

**macOS 的难点是"默认 shell 有两套语义"**。`dscl` 读到的 `UserShell` 是目录服务里的值，`$SHELL` 是当前登录会话的值，两者在切换 shell 后可能不一致；Tabby 只读前者（`app/lib/app.ts:63-69`），并在渲染侧缓存一次（`macDefault.ts:34-44`）。Rust 版建议两者都读、以 `dscl` 为准但把差异记进日志。另外 `--login` 必须带：不带的话 macOS 的 `path_helper` 不跑，`/usr/local/bin` 与 Homebrew 的路径全丢，用户会立刻发现"命令找不到"。

**Linux 的难点是"默认 shell 不等于登录 shell，也不等于终端该跑的 shell"**。`/etc/passwd` 的第七字段是登录 shell，但很多发行版用户实际用 fish/zsh 而 passwd 仍是 `/bin/bash`；Tabby 只用 passwd + `/etc/shells` 两份数据（`linuxDefault.ts:21-47`、`posix.ts:17-41`），没有尊重 `chsh` 之外的选择。Rabby v1 保持一致即可，但要在设置里让"默认 profile 的 shell"显式可选，别让 passwd 成为唯一事实源。

## 3. 与 GPUI 的交叉点

`gpui` 已发布到 crates.io：**0.2.2（2026-10-30）**，仓库 `zed-industries/zed`，可直接作依赖而不再是 git pin。以下为 docs.rs 0.2.2 实测：

| 需求 | GPUI 0.2.2 现状 | 结论 |
|---|---|---|
| 无边框/自绘标题栏 | `WindowOptions.titlebar` + `TitlebarOptions{title,appears_transparent,traffic_light_position,window_decorations}` | 够用；macOS 红绿灯位置可调 |
| 透明/毛玻璃 | `WindowOptions.window_background = Opaque/Transparent/Blurred`；`Window::set_background_appearance` | macOS 够用；Windows 亚克力需 `window-vibrancy`（GPUI 未封装） |
| 多窗口 | `cx.open_window`，`Window::window_handle`/`raw_window_handle` | 多窗口 OK |
| 窗口层级/置顶 | **无** `set_always_on_top`/`set_window_level`（173 个 Window 方法里没有） | Quake 必做平台扩展：Win `SetWindowPos(HWND_TOPMOST)`、macOS `NSWindow.level`、Linux EWMH/`zwlr_layer_shell` |
| 不透明度/跳过任务栏 | **无** `set_opacity`/`set_skip_taskbar` | 同上，走平台扩展 |
| 最小化/最大化/全屏 | `minimize_window`/`zoom_window`/`toggle_fullscreen`/`is_fullscreen` | 够用（注意没有 `hide()`，需自建"隐藏到托盘"路径） |
| 托盘 | 无 | 用 `tray-icon`/`ksni` |
| 全局热键 | 无 | 用 `global-hotkey`/`ashpd` |
| 通知 | 无 | 用 `notify-rust` |
| 文件拖放 | `FileDropEvent{Entered,Pending,Submit,Exited}` + `ExternalPaths` | 类型齐备；具体挂到哪个 element 的 API 待 spike |
| 主题跟随 | `WindowAppearance` + `observe_window_appearance` | 可直接替代 `dark-light`，更省一层 |
| Linux 窗口标识 | `Window::set_app_id` | 对应 Tabby 的 `StartupWMClass`，托盘/任务栏归组要靠它 |

还有一处结构性问题值得提前定调：**GPUI 没有"窗口隐藏"**（方法表里只有 `minimize_window`、`remove_window`、`toggle_fullscreen`，没有 `hide()`）。而 Quake 语义的核心恰恰是"热键 → 隐藏/显示往返"，托盘常驻更是要求"关窗不退出"。这意味着 `rabby-platform` 必须自己持有窗口生命周期：要么在隐藏时销毁 view 并保留状态（等价 `remove_window` + 重建），要么直接用平台原生 `ShowWindow(SW_HIDE)` / `orderOut:` 绕过 GPUI。前者会在重开时丢焦点与滚动位置，后者需要小心 GPUI 的渲染循环仍认为窗口存在——建议先做后者并在 spike 里验证输入焦点能正确回到终端。

判断：**GPUI 的窗口模型覆盖"普通终端窗口"，但不覆盖"Quake 窗口"和"托盘常驻应用"**。这两块要么进 `rabby-platform` 的 `WindowExt` trait，要么接受 v1 不做（推荐 v1 只做"全局热键唤起 + 普通置顶窗口"，跳过 skip-taskbar 与热区贴边）。

## 4. 风险

| 风险 | 影响 | 缓解 | 置信度 |
|---|---|---|---|
| Wayland 全局热键无统一后端 | Linux 用户"Quake 失效"且无报错 | X11 用 XGrabKey；Wayland 走 ashpd portal；两者都不可用时 UI 明示不可用 | 高（`global-hotkey` README 明确 X11 only） |
| 托盘在 Wayland/GNOME 不可靠 | 关窗即"消失" | 单实例 + 二次启动唤起；不默认关窗到托盘 | 中（Tabby 直接对 Linux 关闭托盘，`app.ts:192`） |
| ConPTY 跨提权不可行 | 管理员会话功能缺失 | v1 不做；后续做 helper 内建 ConPTY（照 `UAC.cpp`） | 高 |
| Clink 随包分发（GPLv3） | 许可证冲突 | 不打包，改探测/可选安装 | 高（`extras/clink/LICENSE`） |
| Windows 通知需 AppUserModelID | 通知不显示或归组错 | 安装器写 AUMID + 开始菜单快捷方式；便携版降级为应用内提示 | 中（需实测） |
| macOS 通知需 bundle id + 用户授权 | dev 构建通知静默 | 打包后验证；提供"通知被拒"引导 | 中 |
| `setAlwaysOnTop` 与全屏空间冲突 | Quake 在另一桌面不可见 | 参考 `window.ts:316-318`：`screen-saver` 层 + `visibleOnAllWorkspaces` | 中 |
| VS 探测硬编码 Community | Pro/Enterprise 用户看不到入口 | 改用 `vswhere.exe`（微软官方定位器） | 高（`vs.ts:38`） |
| 便携模式与自动更新互斥 | 便携版更新失败 | 便携检测到就禁用 updater 并跳 Releases（沿用 `updater.service.ts:25-28`） | 高 |

## 5. 推荐决策（可直接落 ADR）

**5.1 shell 目录模型**：`ShellCatalog` 返回 `Vec<Shell>`，字段对齐 Tabby 的 `{id,name,command,args,env,cwd,homeDirArgs,fsBase,shellType,icon,hidden}`；`default` 由 provider 顺序推导，不做特判。`shellType ∈ {unix, powershell, cmd, cygwin}` 决定拖放引号与 OSC 处理。

**5.2 ConPTY**：v1 只用系统 ConPTY（build>=17692），不做 bundled `conpty.dll`；在 `rabby-platform` 里保留 `conpty_dll: Option<PathBuf>` 开关位，等 mouse-tracking 透传被实测证明确实缺失再补。

**5.3 配置根优先级**：`--config-dir` > `RABBY_CONFIG_DIRECTORY` > exe 同目录 `data/`（便携）> 平台标准目录（`dirs::config_dir()`）。与 Tabby 的 `PORTABLE_EXECUTABLE_FILE`+`data/` 双条件等价。

**5.4 单实例**：Unix socket / Windows named pipe 承载"第二实例 argv 转发"，语义覆盖 Tabby 的 `second-instance` + macOS `open-url` 两条入口；`tabby://` 兼容解析器保留（`urlHandler.ts` 的做法是安全样本：只接受白名单 command + 类型化参数）。

**5.5 拖放**：直接沿用"按 shellType 三分支引号"策略（unix `'...'` 转义、PowerShell 单引号幂双写、cmd `^!`/`""`/`%%`），这是 `pathDrop.ts:54-73` 里最容易被低估的实现资产。

**5.6 热键的失败必须可见**：注册失败（被别的软件占用、Wayland 无后端、macOS Secure Input 生效中）都要在设置页该热键旁显示"未生效"及原因，而不是像 Tabby 那样静默 catch。终端用户的期望是"我按了没反应 = 软件坏了"，明确状态能把一类支持成本直接消掉。

**5.6 平台扩展边界**：`WindowExt` 只暴露 5 个方法——`set_always_on_top`、`set_opacity`、`set_skip_taskbar`、`set_visible_on_all_workspaces`、`flash_frame`。GPUI 有能力的不重复封装；没有的走各平台原生。

## 6. 未决问题 / 建议 spike

1. GPUI 侧 `hwnd`/`NSWindow` 获取路径（`raw_window_handle` 是否返回真实句柄）——决定 `window-vibrancy` 与置顶扩展是否可行。
2. GNOME (Mutter) 是否已实现 portal GlobalShortcuts；若未实现，Wayland 上 Quake 只能靠桌面环境自带快捷键。
3. Windows 通知的 AUMID 注册在便携模式下是否可行。
4. macOS Touch Bar：Tabby 仍用 `setTouchBar` 做标签切换（`window.ts:137-147`、`touchbar.service.ts:20-51`），但硬件线已收缩；建议 v1 不做，作为后续增强（此条为判断，未取到 Apple 官方弃用声明）。
5. 通知的"进程完成"判定信号：Tabby 用标签活动流，Rabby 是否有更精确的 OSC 133 语义可用，取决于票 11/13。
