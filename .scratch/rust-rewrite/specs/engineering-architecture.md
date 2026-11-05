# Rabby 工程架构与 crate 划分（草案 v0.1）

> 状态：16 票「工程架构与 crate 划分」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A，本文作为 v1 工程架构基线。
> 输入：11（终端内核/渲染）、12（UI 架构）、13（配置/密钥/迁移）、14（扩展）、15（平台）、09（构建链实测）、08（分发）。
> 一句话：一个 cargo workspace，约 13 个 crate；GPUI 主线程 + 专用 tokio 运行时（SSH/网络/阻塞任务）+ 每会话 PTY reader 线程；UI 线程解析 v1；nightly 固定版本 + 精确 pin + 本地 pathfinder_simd 补丁 + cargo-deny/audit。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A 约 13 个 crate 的 workspace；Q2=A rabby-terminal-view + rabby-ui 拆分；Q3=A GPUI 主线程 + 专用 tokio + 每会话 PTY reader，v1 解析在 UI 线程；Q4=A nightly-2026-11-04 + 精确 pin + vendor pathfinder_simd + cargo-deny/audit；Q5=A 三平台 CI + unit/golden/PTY/SSH/UI/bench；Q6=A thiserror/anyhow + tracing + 脱敏 + 零遥测；Q7=A v1.x 可选 crate/feature + workspace SemVer + ext-api 独立版本。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | crate 粒度 | A. 约 13 个 crate：core/config/secrets/import-tabby/terminal/pty/ssh/session/platform/terminal-view/ui/ext-api/ext-host(v1.x)/app/test-support | B. 5–6 个大 crate：编译快，但边界/测试/许可证/可选依赖难分；C. 每个小功能一个 crate：过度拆分 |
| Q2 | UI 拆分 | A. rabby-terminal-view（TerminalElement/TerminalPane 视图，依赖 gpui-kit + terminal）+ rabby-ui（Workspace/PaneTree/TabStrip/AppShell/settings/command，依赖 terminal-view/session/config/secrets/platform） | B. 单个 rabby-ui 全包：终端渲染与壳耦合，11 的测试边界变差；C. 终端视图塞进 rabby-terminal：terminal 被 GPUI 污染，失去 headless |
| Q3 | 运行时/线程 | A. GPUI 主线程 + 专用 tokio 多线程运行时（SSH/网络/keyring 阻塞任务）+ 每会话 PTY reader 线程 + v1 解析在 UI 线程（256KB/tick）；B. 全部 tokio：GPUI 不适合；C. 全 smol：russh 要 tokio |
| Q4 | 工具链/依赖 | A. rust-toolchain 固定 nightly-2026-11-04；gpui-kit =0.9.0、gpui-pre =0.3.11、wezterm-term git pin、portable-pty =0.9.0、russh =0.65.1 等精确 pin；vendor pathfinder_simd 补丁；Cargo.lock 提交；cargo-deny + cargo-audit | B. 跟随 latest：GPUI/wezterm/russh pre-1.0 会漂；C. stable Rust：GPUI Kit 当前不可行 |
| Q5 | 测试/CI | A. macOS arm64 + Windows x64 + Ubuntu 22.04 x64 三平台 CI；fmt/clippy/test/deny/audit/build release；unit/golden/PTY/SSH/UI headless/bench；B. 只 macOS：Windows/Linux 回归晚发现；C. 手工测试：不可持续 |
| Q6 | 错误/日志/遥测 | A. 库用 thiserror、app 边界 anyhow；tracing + 轮转 + 脱敏；错误码/用户消息；v1 零遥测、崩溃上报仅显式 opt-in；B. 最小 println；C. v1 接 Sentry：与隐私决策冲突 |
| Q7 | feature/版本 | A. v1.x 能力用可选 crate/feature（serial/telnet/ext-host/quake/auto-update），v1 默认不编；workspace SemVer；rabby-ext-api 独立版本；内部 crate publish=false | B. v1 全塞进一个二进制：体积/测试/许可风险；C. 不做版本策略：扩展 ABI 会失控 |

## 1. Workspace 与依赖方向

    Cargo.toml（workspace, resolver=2, edition=2024）
    rust-toolchain.toml（nightly-2026-11-04）
    crates/
      rabby-core            领域类型/ID/事件/错误基类；无 UI、无 IO
      rabby-config          config.toml/state.json/schema 迁移；无 UI
      rabby-secrets         keychain/vault/LegacyTabbyVault；无 UI
      rabby-import-tabby    detect→plan→apply；无 UI
      rabby-terminal        TerminalCore façade + wezterm-term 适配；无 UI
      rabby-pty             portable-pty 传输/启动/退出；无 UI
      rabby-ssh             russh 连接/认证/转发/known_hosts；无 UI
      rabby-session         profile→transport→session 生命周期；无 UI
      rabby-platform        shell catalog/single instance/notifier/clipboard/url/theme/WindowExt；无 GPUI
      rabby-terminal-view   TerminalElement/TerminalPane 渲染；依赖 gpui-kit + terminal
      rabby-ui              Workspace/PaneTree/TabStrip/AppShell/settings/command；依赖 terminal-view/session/config/secrets/platform
      rabby-ext-api         扩展 ABI（manifest/capability/contribution/UI tree 类型）；serde only
      rabby-ext-host        （v1.x）gpui-shell 适配/权限页/contribution registry
      rabby-app             binary/CLI/窗口/单实例/wiring
      rabby-test-support    fixtures/mock platform/fake PTY/SSH test server helpers
    prototype/              09 的 throwaway，不加入 workspace
    docs/adr, GLOSSARY.md, .scratch/

依赖方向（只能向下）：

    core
      ├─ config ── import-tabby
      ├─ secrets ── import-tabby
      ├─ terminal ── terminal-view ── ui
      ├─ pty ── session ── ui
      ├─ ssh ── session
      ├─ platform ── ui
      ├─ ext-api ── ext-host(v1.x) ── app
      └─ ui ── app
    app（wiring）依赖以上全部；test-support 只做 dev-dependency。

- 无 GPUI 的 crate：core/config/secrets/import/terminal/pty/ssh/session/platform/ext-api；必须能在 headless CI 单测。
- 有 GPUI 的 crate：terminal-view/ui/ext-host/app。
- platform 不依赖 GPUI：WindowExt 用 raw-window-handle/opaque handle，GPUI 适配在 ui；15 的 trait 在这里细化。
- ext-host 不进入 v1 默认构建；feature/crate 可选。

## 2. 关键 trait / 接口

    trait TerminalCore        // 11：feed/resize/key/mouse/paste/focus/damage/screen
    trait SessionTransport    // start/write/resize/close/events；local PTY/SSH/serial 各自实现
    trait SessionManager      // profile -> SessionHandle；复用/重连/关闭
    trait ConfigStore         // load/save/watch；schema 迁移
    trait SecretStore         // get/set/delete/list；keychain/vault 后端
    trait Platform            // shell_catalog/single_instance/notifier/clipboard/url/theme/window_ext
    trait ExtensionHost       // v1.x：load/contributions/permissions；gpui-shell/Wasmtime 适配
    trait ContributionRegistry// v1.x：commands/keybindings/menus/settings/panels/terminal hooks

- SessionHandle：writer（输入）、resize、close、reader（bytes/exit）；UI 不直接碰 PTY/SSH 类型。
- AppEvent：SessionOutput/SessionExit/TitleChanged/CwdChanged/Progress/Bell/ConfigChanged/Notification；UI 订阅。
- CommandRegistry：action id + handler + 默认快捷键；扩展贡献走同一注册表。
- 跨层契约：core/config 只依赖 serde/标准库；UI 不直接调 platform 原生；session 不依赖 GPUI。
- 配置/密钥的慢操作（keychain、vault KDF）走 tokio spawn_blocking，UI 用 channel 等结果。

## 3. 运行时与线程模型

- 主线程：GPUI event loop；Workspace/TerminalPane/AppShell 实体；v1 终端解析在此（256KB/tick，11）。
- tokio 运行时：专用后台线程（multi-thread，2–4 worker）；SSH/网络/DNS/HTTP（扩展 v1.x）、keyring spawn_blocking、文件 IO（导入器）。
- PTY reader：每本地会话一个 std::thread；读 64KB chunk，async-channel（有界）送到 UI；退出时发 SessionExit。
- SSH reader：russh 在 tokio 任务里读 channel，转 async-channel 给 UI；session 关闭取消任务。
- 通道：async-channel（有界）；背压策略：PTY 输出满时 reader 阻塞（不丢字节）；UI 按 tick 预算 drain。
- 生命周期：Workspace 拥有 Tab；Tab 拥有 PaneTree；TerminalPane 拥有 SessionHandle+TerminalCore+view；关闭 pane 先 cancel session，再 drop；应用退出 flush 配置/state。
- 扩展（v1.x）：gpui-shell 在 UI 线程；看门狗线程监控超时；Wasmtime 在 tokio 后台；扩展开关/权限事件走 AppEvent。
- 不在 UI 线程做：DNS、SSH 握手、keychain、vault KDF、文件扫描/导入、扩展重活。

## 4. 工具链 / 依赖 / 许可证

- rust-toolchain.toml：nightly-2026-11-04（09 验证 gpui-pre 0.3.11 / cold_path 通过）；MSRV 不是 stable，文档写明。
- 精确 pin（示例）：
  - gpui-kit =0.9.0（传递 gpui-pre =0.3.11）
  - wezterm-term git rev e68a8ec
  - portable-pty =0.9.0
  - russh =0.65.1、russh-sftp =3.0.1（v1.x）
  - keyring =4.2.0、argon2、chacha20poly1305
  - global-hotkey 0.9、ashpd 0.13、notify-rust 4.18、tray-icon 0.26、arboard 3.7、interprocess 2.4、windows-registry 0.100
- [patch.crates-io] pathfinder_simd 指向本地 vendor 补丁（09）；上游修复后移除，记录在 ADR。
- Cargo.lock 提交；CI 用 --locked；release 可用 cargo vendor 归档，保证可复现。
- cargo-deny：允许 MIT/Apache-2.0/BSD/ISC/MPL-2.0；拒绝 GPL/AGPL；Clink 不打包（GPLv3）；serialport MPL-2.0 作为依赖要披露。
- cargo-audit 每次 CI；cargo-vet 可选；依赖升级走 PR + 基准。
- 禁止：运行时 npm 安装、Electron/Node、native dylib 扩展、GPL 代码。

## 5. 测试 / CI 矩阵

- CI 平台：macOS arm64（主）、Windows x64、Ubuntu 22.04 x64；nightly 固定；缓存 target/sccache。
- 门：cargo fmt --check、clippy -D warnings、test --workspace、cargo-deny、cargo-audit、build release。
- 测试层：
  - unit：core/config/secrets/terminal/pty/ssh/session/platform。
  - golden VT：11 的 14 项 + vttest/esctest 子集 + xterm.js 6 参考。
  - PTY：shell 矩阵、resize/SIGWINCH、ConPTY build 分支、pauseAfterExit。
  - SSH：本地 Docker sshd，四种认证、jump、转发、known_hosts；v1.x SFTP。
  - UI：GPUI Kit TestAppContext headless；Workspace/PaneTree/设置/命令；关键截图。
  - 配置/密钥：roundtrip、schema 迁移、Argon2id/XChaCha 向量、LegacyTabbyVault、keyring mock。
  - 导入器：04 的真实 Tabby fixture；dry-run/冲突/回滚/幂等。
  - 扩展（v1.x）：manifest/权限/签名/沙箱/看门狗。
  - 性能：criterion feed/reshape；gpui-fps/自定义 paint；10MB/s@60fps、内存 soak。
- Windows/Linux 专属测试用 cfg/runner 标签；Wayland/热键/托盘手工矩阵。
- 测试支持：rabby-test-support 提供 fake PTY、temp config、mock platform、SSH 容器脚本。

## 6. 错误 / 日志 / 遥测

- 错误：库 crate 用 thiserror 定义错误枚举；app 边界 anyhow + 用户消息；错误码（如 RABBY-TERM-001）便于支持；不把 secret/终端内容写进错误。
- 日志：tracing + tracing-subscriber；per-session span（session id/profile id，不含主机密码）；日志轮转/上限；RUST_LOG；脱敏。
- 遥测：v1 零遥测；崩溃上报只显式 opt-in，且不含终端内容/密钥；Sentry v1 不接。
- panic：库内避免 unwrap；UI 边界 catch_unwind 只用于扩展宿主（v1.x）；终端解析错误转事件，不崩应用。
- 性能指标：启动、首字节、输入延迟、帧时间、内存；基准结果入 CI artifact。

## 7. Feature / 版本 / 发布

- v1 默认 feature：无 extensions/serial/telnet/quake/auto-update；v1.x 用可选 crate + feature 打开。
- workspace 版本 0.1.0；内部 crate publish=false；rabby-ext-api 单独 SemVer（扩展 ABI）。
- release：--locked；三平台产物；签名按 08；Cargo.lock 与 rust-toolchain 入 release notes。
- ADR：docs/adr/0001..0010（终端内核、渲染、UI 拓扑、配置/密钥、扩展、平台、workspace、线程、工具链、测试）。

## 8. 性能预算与基准落点

- 启动 ≤1.5s 冷 / ≤500ms 热；输入 p95 ≤16ms；输出 10MB/s@60fps；8 标签 ≤300MB；搜索 25k <100ms（07）。
- 09 基线：headless 50.8MB/s；UI 33.4MB/s@30fps（1MB/tick）；36 行 CJK 60fps；app 142MB。
- 基准：criterion（terminal feed、config load、vault KDF）；gpui-fps/自定义 paint（帧）；启动/内存脚本；CI 夜间跑，PR 只做 smoke。

## 9. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| GPUI 夜间工具链/快照漂移 | 构建断 | rust-toolchain 固定 + gpui-kit 精确 pin + vendor 补丁 + CI 锁 |
| wezterm-term 0.1/git | API 变更 | façade 隔离 + git pin + 只在必要时 fork |
| tokio 与 GPUI 执行器混用 | 阻塞/死锁 | 专用 tokio runtime；UI 只收 channel；不在 UI 线程 block_on |
| crate 过多/编译慢 | 开发效率 | 13 个边界按职责；CI 缓存；避免微 crate |
| Windows/Linux CI 覆盖不足 | 晚发现 | 三平台 matrix + 手工矩阵 + Wayland runner 标注 |
| 许可证污染 | 合规 | cargo-deny；Clink 不打包；依赖审计 |
| 扩展宿主 ABI 不稳 | v1.x 返工 | v1 不启用；rabby-ext-api 独立版本；gpui-shell 适配层隔离 |

## 10. ADR 清单

- 0001：终端内核 wezterm-term + TerminalCore façade
- 0002：TerminalElement 自绘 + 行缓存/脏行
- 0003：Workspace/PaneTree/TabStrip 自研 UI 拓扑
- 0004：config.toml/state.json + keychain/vault
- 0005：扩展 gpui-shell/Wasmtime + rabby-ext-api + 权限
- 0006：rabby-platform trait + shell 权威源
- 0007：workspace/crate 图
- 0008：运行时/线程模型（GPUI + tokio + PTY threads）
- 0009：工具链/依赖 pin + 许可证
- 0010：测试/CI 矩阵

## 11. 证据

- 11：.scratch/rust-rewrite/specs/terminal-core-rendering.md
- 12：.scratch/rust-rewrite/specs/ui-architecture.md
- 13：.scratch/rust-rewrite/specs/config-secrets-migration.md
- 14：.scratch/rust-rewrite/specs/extension-security.md
- 15：.scratch/rust-rewrite/specs/platform-integration.md
- 09：.scratch/rust-rewrite/specs/09-prototype-report.md
- 08：.scratch/rust-rewrite/specs/platform-distribution.md
