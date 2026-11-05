<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-TITLE: Rabby：用 Rust 重写 Tabby -->
<!-- DSH-STATUS: ready-for-agent -->
<!-- DSH-LABELS: wayfinder:map -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-map -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.224Z -->

## Destination

一份可直接开工的 Rabby 技术蓝图：在 macOS / Windows / Linux 上以 Rust 重写 Tabby 的核心能力（终端、SSH、串口、标签/分屏、配置与密钥、扩展），UI 以 GPUI Kit 为主、NSH 按需；本图以“决策全部锁定、规格可交给实施 agent”为终点，代码由后续实施 effort 执行。

## Notes

- 上游基准：Eugeny/tabby @ `467ce8e`（2026-10-06，提交信息 “xterm 6”），Electron 43 + Angular 15 + xterm.js 6 + node-pty；MIT。
- 产品名暂定 **Rabby**（仓库名）；正式品牌待定。
- 第一性原理拆解：终端应用 = 字节流 ↔ VT 解析/屏幕模型 ↔ 输入编码 ↔ GPU 渲染；会话 = 本地 PTY / SSH / 串口 / Telnet 等传输；再往上才是标签/分屏/设置/恢复/扩展/平台集成。Tickets 按这条脊柱切，不按 Tabby 的 Angular package 边界切。
- 范围（2026-11-05 用户逐票确认；第一轮提问超时后先按推荐项画图，后续均已拍板）：
  - 终点 = 可开工蓝图，不在本 effort 写产品代码；
  - v1 = 终端 + 本地 shell + SSH + 标签/分屏 + 设置/主题 + 拓扑级会话恢复；SFTP/串口/Telnet/Zmodem/扩展市场放 v1.x；
  - 平台 = 桌面三平台优先，Web 后续，移动暂不做；
  - 扩展 = 新 Rust/WASM + JS 扩展机制，不兼容 Tabby Angular 插件；
  - 迁移 = 支持从 Tabby 一次性导入 profile/SSH/known_hosts/配色/快捷键；会话恢复不可迁（04/13）；
  - UI = GPUI Kit 为主，NSH 仅按需（Web/移动/运行时可下发 TSX）。
- 执行约定：每个会话按 ticket 的 `wayfinder:<type>` 执行；research 用 /research，prototype 用 /prototype，grilling 用 /grilling + /domain-modeling。
- 研究笔记放 `.scratch/rust-rewrite/research/`，只链接不粘贴；本会话未获远端写授权，不 push 研究分支，也不把研究产物合并进产品代码。
- 术语与 ADR 按需创建：根 `GLOSSARY.md`、`docs/adr/`，只在决策真正落定时写；不预建空文件。
- 依赖形状（2026-11-05 校验，无环）：W0 七张 research 并行（01–06、10）；W1 并行 07/09/13/14/15；W2 并行 08/11；W3 12；W4 16；W5 17。串行只留在真有先后要求处：07 等全部 research；08 等 07；12 等 08+09；16 等 11–15；17 等 07+08+16。
- 蓝图完成（2026-11-05）：17/17 子票闭环；规格索引见 specs/README.md；实施从 M0 开始（specs/delivery-plan.md）。

## Decisions so far

<!-- 研究票闭环后回填：一行一票，名称 + 一句话结论 + 链接 -->
- [Tabby 功能与插件面盘点](issues/01-tabby-功能与插件面盘点.md): Tabby 的功能面不是重写的难点，隐性行为和插件生态才是；SSH 层上游已用 Rust（russh）实现，可直接复用。
- [Tabby 配置、密钥与会话兼容面](issues/04-tabby-配置-密钥与会话兼容面.md): 迁移面只有三处真源：config.yaml（+ .backup）、userData/plugins、Electron localStorage(LevelDB)，没有独立密钥库；导入器应做字段级翻译并以 Rabby 默认值兜底（Tabby …
- [Rust 网络与会话栈选型](issues/05-rust-网络与会话栈选型.md): SSH/SFTP 选 russh 0.65.x + russh-sftp 3.0.x；串口 serialport 4.10.1 + 自建热插拔；Telnet 用 telnet 0.2.5 或自写 IAC codec（v1.x）；Zmodem…
- [平台集成细节：shell、热键、通知、便携](issues/10-平台集成细节-shell-热键-通知-便携.md): 结论：平台集成独立成 rabby-platform crate，shell 发现走「注册表/dscl/getpwuid 为主的单一权威源」，Windows 只做 ConPTY，Quake/热键三平台分级（Wayland 走 ashpd Gl…
- [扩展运行时选型：WASM 组件 vs JS 宿主](issues/06-扩展运行时选型-wasm-组件-vs-js-宿主.md): 主运行时选 gpui-shell（QuickJS + 声明式 UI 描述树 + capability manifest），兜底 Wasmtime + Component Model 只承载无 UI 的重活。要点：① Tabby 插件实为全权…
- [GPUI / GPUI Kit / NSH 能力评估](issues/02-gpui-gpui-kit-nsh-能力评估.md): 一句话结论：v1 用 GPUI Kit 0.9 做桌面外壳 + 自研 terminal，NSH 不进 v1 主路径。要点：① GPUI 无稳定 crate：官方 gpui 停在 0.2.2，生态实际用 gpui-pre 0.3.11（Zed…
- [Rust 终端引擎与 PTY 选型](issues/03-rust-终端引擎与-pty-选型.md): 推荐：内核 wezterm-term（git pin + 自建 TerminalCore façade）、PTY portable-pty、GPUI 自绘网格；若 v1 砍掉图片协议与 bidi，则改选 alacritty_terminal…
- [v1 产品边界与验收口径](issues/07-v1-产品边界与验收口径.md): v1 = 可日报的 Rust 桌面终端：终端 + 本地 shell + SSH + 标签/分屏 + 设置/主题 + 拓扑级会话恢复 + Tabby 迁移；SFTP/串口/Telnet/Zmodem/Quake/托盘/全局热键/插件/PTY …
- [平台、分发与品牌](issues/08-平台-分发与品牌.md): 平台：macOS 先行 → Windows → Linux；GitHub Releases 直发；macOS dmg / Windows msi+便携 zip / Linux AppImage+deb；开发期未签名，macOS 公开 bet…
- [终端垂直切片原型](issues/09-终端垂直切片原型.md): 原型结论：GPUI Kit 0.9 + wezterm-term + portable-pty 在 macOS arm64 跑通；headless release 50.8 MB/s、25k scrollback RSS 36.5MB；GP…
- [UI 架构与 NSH 边界](issues/12-ui-架构与-nsh-边界.md): UI 架构：自研 Workspace/PaneTree/TabStrip（Kit 只做外观零件）；v1 顶部标签 + 四向嵌套分屏 + 最大化，四边标签/Quake/多窗口放 v1.x；TerminalPane 拥有 TerminalCor…
- [终端内核与渲染边界](issues/11-终端内核与渲染边界.md): 内核：wezterm-term git pin + TerminalCore façade，暂不 fork；v1 UI 线程解析 + 256KB/tick 字节预算，worker 放 v1.x；自研 TerminalElement（shap…
- [配置、密钥与迁移](issues/13-配置-密钥与迁移.md): 配置：config.toml 完整规范化 + state.json；显式 schema_version + 类型化 profile + options map；密钥默认 OS keychain、可选 vault；vault 用 Argon2…
- [扩展机制与安全边界](issues/14-扩展机制与安全边界.md): 扩展：gpui-shell 主运行时 + Wasmtime 无 UI 重活兜底，Rabby 自建 rabby-ext-api；v1 不做第三方运行时，只做 ABI/权限设计 + 宿主内置；manifest + 默认拒绝 + 三态授权 + s…
- [平台集成与系统能力](issues/15-平台集成与系统能力.md): 平台：独立 rabby-platform crate + trait；shell 每类一个权威源 + ProbeResult + TTL；Windows 只用系统 ConPTY、不 bundle dll/不做 winpty、UAC v1.x…
- [工程架构与 crate 划分](issues/16-工程架构与-crate-划分.md): 工程架构：约 13 个 crate 的 workspace（core/config/secrets/import-tabby/terminal/pty/ssh/session/platform/terminal-view/ui/ext-ap…
- [交付分期与里程碑](issues/17-交付分期与里程碑.md): 交付：5–6 人、9–12 个月到 v1.0；M0 骨架 → M1 本地终端（macOS alpha）→ M2 SSH+迁移+Windows alpha → M3 三平台 beta → M4 v1.0；beta 门槛 P0 全过/迁移 0 …

## Not yet specified

- Web / 远程网关的具体形态（取决于平台决策；Tabby Web 是 server-side PTY/SSH gateway）
- 移动端（NSH iOS/Android）的产品形态与交互（仅当平台决策扩展到移动）
- 插件市场、签名分发与审核运营
- 自动更新渠道、代码签名与公证（macOS notarization / Windows signing / Linux 仓库）
- 性能预算与基准口径（启动、输入延迟、吞吐、内存、大 scrollback）
- 遥测、隐私与崩溃上报策略
- 国际化 / 本地化流程（Tabby 用 gettext/Crowdin）
- 无障碍验收标准（VoiceOver / NVDA / Orca）
- AI / MCP 类扩展（Tabby 已有 mcp-server 插件）
- 与上游 Tabby 的长期功能对齐机制
- 测试策略细节（headless UI、PTY 集成、SSH 集成、视觉回归）

## Out of scope

- 在 Rabby 内运行 Tabby 的 Angular/Electron 插件（新扩展机制替代）
- 嵌入 Electron/Node 运行时
- v1 的移动端与 Tabby Web 对齐（后续 effort）
- 重写 Clink/shell 本体（只做集成）
- Tabby 云同步 / 账户体系
- 实际编码、仓库工具链与发版流水线（本图为蓝图；若目的地改为执行，另开实施 effort）
- 品牌/商标的法律事务（仅记录命名决策）

## 进度

- 进度：100%（2026-11-05T11:37:36+08:00）：17/17 子票闭环；蓝图完成；Decisions so far 17 行；specs/ 10 份决策文档；可做最终核对并标记完成。
- 进度：99%（2026-11-05T11:35:02+08:00）：16 工程架构闭环，决议在 specs/engineering-architecture.md。17 已解锁；当前真实 frontier = 17 一张；走完即目的地。
- 进度：97%（2026-11-05T11:31:00+08:00）：15 平台集成闭环，决议在 specs/platform-integration.md。16 已解锁；当前真实 frontier = 16 一张；17 收尾。
- 进度：95%（2026-11-05T11:28:37+08:00）：14 扩展/安全边界闭环，决议在 specs/extension-security.md。当前真实 frontier = 15 一张；16 等 15，17 收尾。
- 进度：90%（2026-11-05T11:26:12+08:00）：13 配置/密钥/迁移闭环，决议在 specs/config-secrets-migration.md。当前真实 frontier = 14/15 两张并行；16 等这两张，17 收尾。
- 进度：85%（2026-11-05T11:22:57+08:00）：11 终端内核/渲染闭环，决议在 specs/terminal-core-rendering.md。当前真实 frontier = 13/14/15 三张并行；16 等这三张，17 收尾。
- 进度：75%（2026-11-05T11:19:20+08:00）：12 UI 架构闭环，决议在 specs/ui-architecture.md。当前真实 frontier = 11/13/14/15 四张并行；16 等这四张，17 收尾。
- 进度：65%（2026-11-05T11:14:34+08:00）：09 终端垂直切片原型闭环，报告在 specs/09-prototype-report.md。11/12 已解锁；当前真实 frontier = 11/12/13/14/15 五张并行；16/17 在后。
- 进度：55%（2026-11-05T10:19:29+08:00）：08 平台/分发/品牌已闭环，决议在 specs/platform-distribution.md。当前真实 frontier = 09/13/14/15；12 与 11 都等 09，16/17 在后。
- 进度：45%（2026-11-05T10:15:45+08:00）：07 v1 边界与验收口径已闭环，基线在 specs/v1-scope.md。当前真实 frontier = 08/09/13/14/15 五张并行；11 等 09，12 等 08+09，16/17 在后。
- 进度：35%（2026-11-05T09:22:02+08:00）：7/7 research 全部闭环，Decisions so far 已回填 7 行。按逐张读回，当前真实 frontier = 07 / 09 / 13 / 14 / 15 五张并行；08 等 07，11 等 09，12 等 08+09，16/17 在后。
- 进度：25%（2026-11-05T09:20:45+08:00）：6/7 research 闭环（01/02/04/05/06/10）；03 终端内核仍在调研。按逐张读回，当前真实 frontier 是 13/14/15 三张决策票并行可接；07/09 等 03。
- 进度：20%（2026-11-05T09:18:56+08:00）：5/7 research 闭环（01/04/05/06/10）；02 笔记已落盘待关票，03 终端内核仍在调研；Decisions so far 已回填 5 行。
- 进度：15%（2026-11-05T09:17:51+08:00）：3/7 research 已闭环（01 Tabby 功能面、04 配置/密钥、05 网络栈），4 张调研中（02 UI 框架、03 终端内核、06 扩展运行时、10 平台集成）；Decisions so far 已自动回填 3 行。
- 进度：5%（2026-11-05T09:12:44+08:00）：地图与 17 张子票已建；7 张 research 票（01/02/03/04/05/06/10）已认领并派出子代理并行调研；其余 10 张待研究结论解锁。
