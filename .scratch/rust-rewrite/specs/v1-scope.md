# Rabby v1 范围与验收口径（草案 v0.1）

> 状态：07 票「v1 产品边界与验收口径」的决议，2026-11-05。用户已确认 §0 的 Q1–Q5 全部按推荐 A，本文作为 v1 验收基线。
> 输入：research 01（功能面/隐性行为）、02（UI 框架）、03（终端内核）、04（配置/密钥）、05（SSH/串口）、06（扩展）、10（平台集成）；用户 2026-11-05 确认的高层范围。
> 一句话：v1 是一台可日用的 Rust 桌面终端——本地 shell + SSH + 标签/分屏 + 设置/主题 + 拓扑级会话恢复 + Tabby 迁移；插件、SFTP、串口、Telnet、Zmodem、Quake/托盘、Web/移动不进 v1。

## 0. 已拍板（2026-11-05：Q1–Q5 全部按推荐 A）

> 决议：Q1=A 核心 SSH（X11/dynamic P1）；Q2=A 普通窗口（Quake/托盘/热键 v1.x）；Q3=A 五页设置（CSS 主题/vibrancy/插件页 v1.x）；Q4=A 暂定性能目标 + 09 校准；Q5=A v1 无 P0 缺项，18 条 = 12 P0 + 6 P1。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | SSH v1 边界 | A. 核心 SSH：none/password/publickey/keyboard-interactive/agent（含 Pageant）、jump host、local+remote 转发、known_hosts、agent forwarding、连接复用、ProxyCommand/SOCKS/HTTP 代理；X11 与 dynamic SOCKS5 放 v1.x | B. 完整对齐 Tabby（含 X11/dynamic）：周期 +2–4 周，X11 三平台验证成本高；C. 更窄（只做认证+jump host）：转发全放 v1.x，削弱可日用 |
| Q2 | 窗口与系统能力 | A. 普通窗口 + 标签/分屏 + 最小系统集成（单实例、URL scheme、文件拖放、通知默认关）；Quake、托盘、全局热键放 v1.x | B. 加 Quake/托盘/全局热键进 v1：GPUI 没有这些 API，要自研平台扩展，风险最高；C. 只做窗口，单实例/拖放也延后：体验不完整 |
| Q3 | 设置与主题 | A. 终端外观（字体/配色/光标/滚动）、profiles（本地/SSH）、快捷键、常规、配色方案库；CSS 主题、vibrancy、自定义 CSS、插件设置页放 v1.x | B. 完整对齐 Tabby 设置：范围膨胀；C. 最小设置：多 profile 与迁移不好落 |
| Q4 | 性能验收 | A. 先定暂定目标 + 09 原型校准（见 §3.3）；v1 回归线 = 同场景不差于 Tabby 10% | B. 只写体感不差于 Tabby：无法验收；C. 更严格（冷启 ≤1s、输入 p95 ≤8ms、内存 ≤200MB）：需 09 数据再定 |
| Q5 | P0/P1 政策与 18 条划分 | A. v1 不允许 P0 缺项；P1 可在 v1.x 补，但必须在兼容矩阵标注；18 条 = 12 条 P0 + 6 条 P1（见 §3.1） | B. 允许少量 P0 缺项，标注即可：稀释可日用承诺；C. 主观判断：无法验收 |

## 1. v1 定义与非目标

### 1.1 v1 必须做到

- 在 macOS 14+、Windows 10 1809+（ConPTY 17692+）、Linux X11/Wayland 上启动、开本地 shell、开 SSH、开标签/分屏、改设置/主题、重启后恢复标签拓扑、从 Tabby 导入配置。
- 终端行为对齐 xterm.js 6 的默认契约（§3.2），并把 01 的 18 条隐性行为逐条写成验收项（§3.1）。
- 范围：终端、SSH、设置、迁移、平台基础集成。插件/扩展运行时只做设计（14 票），实现进 v1.x。

### 1.2 v1 明确不做

- 不运行 Tabby 的 Angular 插件；不做第三方扩展市场（14 票只出机制设计）。
- 不做 SFTP、串口、Telnet、Zmodem（均 v1.x）。
- 不做 Quake、托盘、全局热键（v1.x）。
- 不做 Web 版、移动端（地图 Out of scope）。
- 不做 PTY re-attach 式会话恢复（P1；v1 只恢复标签/分屏拓扑）。
- 不做 CSS 主题、vibrancy、自定义 CSS、插件设置页（v1.x）。
- 默认不收集遥测；崩溃上报只在用户显式开启时（v1 可不实现）。
- 不做自动更新（08 票定分发形态；v1 用安装包手动更新）。

## 2. 范围表（P0/P1/P2）

优先级：P0 = v1 必须，缺项不发布；P1 = v1.x 首个跟进版本，v1 可缺但必须在兼容矩阵标注；P2 = 后续/不承诺。

| 域 | 能力 | 优先级 | 验收 | 来源 |
|---|---|---|---|---|
| 终端 | VT220+ 扩展、alt-screen、鼠标追踪、bracketed paste、OSC 8/52/1337-cwd、OSC 9;4 进度 | P0 | vttest/esctest 子集 + 14 项兼容契约 | 01 §1.1/§3、03 §87 |
| 终端 | 真彩色、16 色主题、亮暗模式、配色方案库 | P0 | 截图对比 + 配色导入 | 01 §1.1 |
| 终端 | 25k scrollback、resize reflow | P0 | 大文件/25k 行滚动；resize 不丢行 | 01 §1.1、03 |
| 终端 | 搜索（正则/全词/大小写） | P1 | 25k 行正则搜索 <100ms | 01 §1.1 |
| 终端 | 字体/字号/行距/fallback、CJK 双宽、emoji、IME 预编辑 | P0 | 三平台 CJK/IME 手工矩阵 + 网格快照 | 01 §1.1、02 §4 |
| 终端 | 选中/复制（copyOnSelect/右键/中键）、粘贴三件套 | P0 | 粘贴用例 + 平台换行表 | 01 §3 |
| 终端 | 分屏（四向嵌套、聚焦、最大化单 pane）、标签（拖动/重排/重命名/关闭/MRU） | P0 | UI 集成测试 + 手工矩阵 | 01 §1.1 |
| 终端 | 进度检测、8MB 预缓冲、UTF-8 分块、Backspace 编码、Bell 三态、配置双写 | P0 | §3.1 对应行 | 01 §3 |
| 终端 | SIXEL/iTerm2 图片、连字、copyAsHTML | P1 | 图片/连字 golden；copyAsHTML 手工 | 01 §3、03 |
| 本地 | PTY（Unix openpty / Windows ConPTY）、cwd 继承、退出行为、resize/SIGWINCH | P0 | 三平台 shell 冒烟 + resize 用例 | 03 §59、10 §1.2 |
| 本地 | Shell 探测：macOS/Linux 默认；Windows CMD/PS/PS Core/WSL/Git-Bash/Cygwin/MSYS2/Cmder/VS | P0 | 每类 shell 启动 + 环境变量检查 | 01 §1.2、10 §1.1 |
| 本地 | Windows 注册表环境重建、WSL 注册表枚举、Clink 探测（不随包分发） | P0 | Windows 手工矩阵 | 01 §3、10 §0 |
| SSH | 认证：none/password/publickey（多 key+口令）/kbd-interactive/agent（Unix socket、Windows OpenSSH pipe、Pageant） | P0 | 与 Tabby 同服务器对照 | 05 §二/§三 |
| SSH | jump host、local/remote 转发、agent forwarding、known_hosts（Tabby JSON + OpenSSH 双读）、连接复用、ProxyCommand/SOCKS/HTTP | P0（Q1） | 同一套 Linux 测试服务器 + Tabby 对照 | 05 §四 |
| SSH | X11、dynamic SOCKS5 | P1（Q1） | XQuartz/VcXsrv + SOCKS 客户端 | 05 §四 |
| 设置 | 常规、终端外观、profiles（本地/SSH）、快捷键、配色方案 | P0（Q3） | 设置项读写 + 重启生效 | 01 §1.3 |
| 主题 | 亮/暗/跟随系统、16 色方案、透明度（GPUI 支持范围内） | P0 | 主题切换截图 | 01 §1.3 |
| 恢复 | 标签/分屏拓扑、profile 快照、recentProfiles；崩溃后恢复 | P0 | 关窗/崩溃/重启用例 | 04 §5 |
| 恢复 | PTY re-attach（接管原进程） | P1（用户已定） | 上游行为对照；v1 不承诺 | 01 §3、04 §5 |
| 迁移 | config.yaml(+.backup) 到 profiles/groups/defaults/known_hosts/配色/快捷键/可映射终端设置；vault 一次性解密；dry-run + 报告 | P0 | 真实 Tabby 配置 fixture + 0 P0 数据丢失 | 04 §7/§9 |
| 迁移 | 明文密码/登录脚本默认不导入，逐条确认；密钥进 OS keychain；不写回 Tabby 文件 | P0 | 安全用例 + 零明文落盘 | 04 §7/§8 |
| 平台 | 单实例、URL scheme、文件拖放、通知默认关、配置根优先级、便携模式（P1） | P0（便携 P1） | 三平台手工矩阵 | 10 §5 |
| 平台 | 托盘、全局热键、Quake | P1 | 三平台分级；Wayland 失败可见 | 02 §4、10 §5 |
| 扩展 | 扩展机制设计（gpui-shell + Wasmtime 兜底、权限矩阵） | P1（设计在本图 14 票） | v1.x 实现 | 06 §6/§9 |
| 其他 | SFTP、串口、Telnet、Zmodem | P1（v1.x） | 05 的验收基线 | 05 §五–§八 |
| 其他 | Web、移动、自动更新、i18n 扩展、CSS 主题/vibrancy | P2 | 另开 effort | 地图 Out of scope |

## 3. 终端验收

### 3.1 18 条隐性行为（全部是验收项）

| # | 行为 | 优先级 | 验收方法 |
|---|---|---|---|
| 1 | 进度检测（正则、非 alt-screen、>0 且 ≤100、失配清空） | P0 | 输出含 25%/100%/0% 的 golden；断言标签进度属性 |
| 2 | 8MB 预缓冲（超限丢最旧 + 提示行） | P0 | 未聚焦标签灌 9MB；断言截断提示与内存上限 |
| 3 | 粘贴三件套（bracketed、多行确认、平台换行归一） | P0 | 三平台粘贴矩阵 + 多行确认对话框 |
| 4 | copyAsHTML（默认开） | P1 | 复制后剪贴板含 text/html；可关 |
| 5 | Unicode 11 宽度（emoji/双宽/ZWJ） | P0 | 宽度表 golden + 与 xterm.js 对照 |
| 6 | SIXEL 图片（默认开） | P1 | SIXEL/iTerm2 样例；pixelLimit 16M |
| 7 | 连字（默认关、Web 禁用） | P1 | 开关 + 字体矩阵；默认关 |
| 8 | OSC cwd 上报、OSC 52 写剪贴板 | P0 | 新标签继承 cwd；剪贴板策略可关 |
| 9 | UTF-8 分块保护 | P0 | 随机切 chunk 注入多字节字符；无乱码 |
| 10 | Backspace 编码四档 | P0 | 四档配置 + 与 xterm 对照 |
| 11 | Bell 三态 + 任务栏闪烁 | P0 | off/visual/audible + 平台闪烁 |
| 12 | PTY re-attach 式恢复 | P1 | 上游行为对照；v1 只恢复拓扑 |
| 13 | Windows 环境变量重建（HKLM/HKCU/Volatile、Path 追加） | P0 | Windows 手工矩阵 |
| 14 | ConPTY 双路径（bundled dll 到系统回退） | P1 | v1 只用系统 ConPTY；保留开关位 |
| 15 | Clink（不随包分发，探测已装） | P0 | Windows 已装/未装两态 |
| 16 | WSL 细节（Lxss 枚举、wsl 共享路径、--cd 门槛） | P0 | WSL1/WSL2 手工矩阵 |
| 17 | 便携模式（exe 同目录 data/） | P1 | 三平台便携启动 |
| 18 | 配置双写（config + .backup） | P0 | 保存/崩溃/回滚用例 |

### 3.2 xterm.js 6 兼容契约（14 项）

| 项 | 契约 | 优先级 |
|---|---|---|
| kitty keyboard | 默认 false（opt-in） | P0 |
| win32-input-mode | 默认 false | P0 |
| 配色查询 CSI ? 996n / DECSET 2031 | 默认 true | P0 |
| 光标闪烁 DECSET 12 | 默认忽略（quirks） | P0 |
| ConPTY 补偿 | buildNumber < 21376 关 reflow；行尾推断 | P0（Windows） |
| bracketed paste | DECSET 2004；宿主编码 | P0 |
| OSC 52 | 宿主策略，可关 | P0 |
| OSC 1337 CurrentDir | 只认 CurrentDir= | P0 |
| OSC 8 hyperlink | 点击确认/打开策略 | P0 |
| search/serialize | 屏幕搜索 + ANSI 快照 | P1 |
| 图片 | SIXEL + iTerm IIP；pixelLimit | P1 |
| 宽字符 | Unicode 11 表 | P0 |
| 渲染选项 | minimumContrastRatio / drawBoldTextInBrightColors | P1 |
| 字体 fallback/emoji | 彩色字形、ZWJ | P0 |

### 3.3 性能门槛（暂定，09 原型校准）

参考机：Apple M1/16GB；Ryzen 5/16GB；Windows 10 22H2。

| 指标 | v1 目标 | 备注 |
|---|---|---|
| 冷启动到可输入 | ≤1.5s | 含首帧；09 实测后可调 |
| 热启动到可输入 | ≤500ms | 二次启动 |
| 按键到显示 p95 | ≤16ms | 60Hz 一帧；stretch ≤8ms |
| 输出吞吐 | ≥10MB/s 持续，60fps 不掉帧，字节不丢 | yes/cat 大文件 |
| 25k scrollback 内存 | ≤300MB（8 个标签） | 基线 ≤150MB |
| resize 80x24 到 200x60 | <100ms，无丢行 | ConPTY 老 build 分支 |
| 搜索 25k 行 | <100ms | 正则 |
| 8 小时 soak | 无崩溃、无泄漏（RSS 增长 <10%） | 本地脚本 |

## 4. SSH 验收（v1 核心，Q1 定稿）

| 能力 | v1 | 验收 |
|---|---|---|
| 认证 none/password/publickey（多 key+口令）/kbd-interactive/agent | P0 | 与 Tabby 同服务器对照；错误降级顺序一致 |
| Agent：Unix socket / Windows OpenSSH pipe / Pageant | P0 | 三平台 agent 矩阵；管道优先、Pageant 兜底 |
| Jump host（单跳/多跳） | P0 | 跳板机集成测试 |
| local/remote 转发 | P0 | 端口连通性 + 冲突报错 |
| agent forwarding | P0 | 远端 ssh-add -l |
| known_hosts：Tabby JSON + OpenSSH 双读 | P0 | 首次确认、变更告警、哈希条目 |
| 连接复用池 | P0 | 同主机 5 tab 只建 1 条 TCP；引用计数 |
| ProxyCommand / SOCKS / HTTP 代理 | P0 | 代理服务器集成测试 |
| X11 / dynamic SOCKS5 | P1 | XQuartz/VcXsrv + SOCKS5 CONNECT |
| 算法白名单（runtime 枚举与 profile 取交集） | P0 | 与上游 algorithms.ts 对照 |
| 错误可见（转发失败、host key 变更、认证失败） | P0 | 通知/对话框，不静默 |

SSH 验收基线：同一台 Linux 服务器跑 Tabby 与 Rabby 对照；四种认证、带口令私钥、跳板机、local/remote/dynamic、X11（有 X server 时）、known_hosts 首次与变更。

## 5. UI / 设置 / 主题 / 恢复

- 标签/分屏：四向嵌套、拖动重排、重命名、关闭、MRU 循环、焦点切换；分屏比例持久化；最大化单 pane。验收：UI 集成测试 + 三平台手工。
- 设置页：常规、终端外观、profiles（本地/SSH）、快捷键、配色方案；设置写入后即时生效/重启生效明确。验收：每页字段读写 + 迁移后配置正确。
- 快捷键：多 chord、平台默认表、冲突检测；注册失败必须可见（10 §5.6）。验收：表格 + 冲突用例。
- 主题：亮/暗/跟随系统、16 色、字体/透明度（GPUI 支持范围内）；CSS 主题/vibrancy 不做。验收：截图 + 配色导入。
- 会话恢复：标签/分屏拓扑 + profile 快照 + recentProfiles；崩溃后恢复；不接管旧进程。验收：关窗/崩溃/重启三用例。
- 无障碍：键盘可达、AccessKit 语义冒烟（P1 完整审计）。

## 6. 迁移验收

- 输入：Tabby config 目录（平台标准 / TABBY_CONFIG_DIRECTORY / 便携 data）；只读，不写回。
- 输出：ImportPlan（强类型、可 dry-run）+ 导入报告（成功/跳过/需重输/无等价物）。
- 映射：profiles（local/ssh）、groups、profileDefaults、known_hosts、配色、快捷键（冲突检测）、可映射 terminal 设置。
- 密钥：vault 口令一次性解密到 RabbyVault（Argon2id + XChaCha20-Poly1305）或 OS keychain；明文密码/登录脚本默认不导入，逐条确认；零明文落盘/日志。
- 不迁：tabsRecovery（xterm 私有序列化）、plugins、appearance.css、configSync/analytics/electronFlags/hacks。
- 验收：真实 Tabby 配置 fixture（含 vault、SSH、配色、快捷键）到 0 P0 数据丢失；所有跳过项在报告中可见；用户可在应用前审阅/取消。

## 7. 平台与分发边界

- 平台：macOS 14+；Windows 10 1809+（ConPTY 17692+，reflow 21376+ 分支）；Linux X11 + Wayland（Wayland 全局热键 v1 不做）。
- 系统：单实例、URL scheme、文件拖放、通知默认关、配置根优先级（--config-dir 优先，其次 RABBY_CONFIG_DIRECTORY，其次便携 data/，最后平台标准）。
- 分发（08 票定）：v1 至少产出三平台可运行安装包；签名/公证/自动更新不在 07 的验收内。
- 不做：托盘、全局热键、Quake（P1）；Web/移动（Out of scope）。

## 8. 成功指标

1. 可日用：连续 1 周作为主终端；8 小时 soak 无崩溃；vim/htop/ssh/tmux 正常。
2. 可迁移：真实 Tabby 配置导入 0 P0 数据丢失；跳过项 100% 报告。
3. 性能：§3.3 全部达标（09 校准后定稿）。
4. 兼容：vttest/esctest 子集通过；§3.2 14 项无 P0 差异。
5. 质量闸：v1 发布时无 P0 缺项；P1 缺项在兼容矩阵逐条标注。

## 9. 验收方法与依赖

- 测试层：单元/golden（VT 字节流到屏幕快照/ANSI）、集成（PTY/SSH 真服务器）、UI headless（GPUI Kit TestAppContext）、三平台手工矩阵、性能基准（criterion）。
- 依赖：09 原型给出性能/IME/CJK 实测；11 定内核；08 定分发；12 定 UI 架构；13 定配置/vault；14 定扩展；16 定 crate/测试架构。
- 本草案的 5 个拍板项（§0）确认后，07 关闭，兼容矩阵进入 v1 验收清单。

## 10. 证据索引

- 01：.scratch/rust-rewrite/research/01-tabby-feature-plugin-audit.md
- 02：.scratch/rust-rewrite/research/02-gpui-kit-nsh-capability.md
- 03：.scratch/rust-rewrite/research/03-terminal-engine-pty.md
- 04：.scratch/rust-rewrite/research/04-tabby-config-vault-compat.md
- 05：.scratch/rust-rewrite/research/05-rust-ssh-serial-stack.md
- 06：.scratch/rust-rewrite/research/06-extension-runtime.md
- 10：.scratch/rust-rewrite/research/10-platform-integration.md
