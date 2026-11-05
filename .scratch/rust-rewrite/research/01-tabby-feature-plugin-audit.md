# Tabby 功能与插件面盘点（research 01）

**票号** 01 · **上游** Eugeny/tabby @ `467ce8e`（2026-10-06，commit "xterm 6"，nightly 1.0.231）· **来源** 只读源码 / README / HACKING，行号对应该 commit。

**基线事实**：Electron 43 + Angular 15 + xterm.js 6.0 + node-pty 1.2.0-beta.8 + russh 0.1.38，Webpack 打包，MIT 许可。内置 13 个包统一标 `tabby-builtin-plugin`、版本号齐刷刷是 `1.0.231-nightly.0`，随安装包分发而不是从 npm 现取（`app/src/plugins.ts`）。这意味着上游根本不存在"稳定版插件 API"这回事——第三方插件作者面对的是每天变的 nightly；Rabby 想做兼容层，等于去对齐一个自己都不承诺稳定的接口。配置是单文件 YAML（`config.yaml` + `.backup`），24 个 `.po` 语言文件 + `app.pot` 模板。本盘点只覆盖桌面三平台，Web 与移动不在范围内。

---

## 0. 决策摘要

- **SSH 不用重写，上游已经用 Rust 写完了。** `tabby-ssh` 从 `session/ssh.ts:21` 起 `import * as russh from 'russh'`，依赖在 `app/package.json:35`（`"russh": "0.1.38"`）。认证、channel、SFTP 文件类型、X11、jump host、SOCKS/HTTP 代理、Pageant、公钥解析全部调用 russh（`session/{ssh,sftp,shell,shellChannel}.ts`、`services/ssh.service.ts`、`algorithms.ts` 共 6 个文件）。**对 Rabby 的含义：SSH 是重写里"风险最低、收益最确定"的一块**——直接依赖 russh crate，连 N-API 绑定层和 Node 事件循环一起删掉，行为语义还能和上游逐条对齐。

- **重写主体是 VT 内核、UI 框架和隐性行为，不是协议栈。** 内置包约 26.4k 行 TS：core 8830 / terminal 5156 / ssh 3617 / electron 3374 / settings 2066 / local 1501 / serial 600 / telnet 611 / web 385 / plugin-manager 322。其中没有现成 Rust 等价物的是 xterm.js 6 的行为面（§3）与 Angular 的 DI 扩展点（§2.2）。

- **插件 API 不可兼容，也不该兼容。** 扩展面是 31 个 Angular `abstract class` + DI `multi: true` provider（§2.2），扩展单元是一个 `NgModule` 类（`HACKING.md:100`），没有跨语言 ABI。做兼容层的唯一实现方式是继续内嵌 Electron/Angular——与地图的 Out of scope 直接冲突。

- **v1 的真风险是"隐性行为"而不是"功能清单"。** README 的 13 条功能 30 分钟能抄完；§3 的 18 条才是用户第一周就会说"这不是 Tabby"的地方（progress detection、8MB 预缓冲、copyAsHTML、Unicode 11 宽度、sixel、PTY re-attach……）。

- **v1 边界建议**：维持地图假设，但把 §3 全部写成验收项；串口 / Telnet 是上游最薄的两个包（600 / 611 行），Zmodem 只是 terminal 包里的一个 decorator（`features/zmodem.ts` 约 330 行），都可延后；**PTY re-attach 式会话恢复建议降到 P1**——上游自己也只是"能拿到旧 PTY 句柄才恢复"（§3 第 12 条），把它当 v1 承诺会给 Rust 端凭空加一个高难度约束。

---

## 1. 桌面功能面 parity matrix

复杂度为"用 Rust + 成熟 crate 重做"的估算，不是上游代码量。

### 1.1 终端内核

| 功能 | 用户可见行为 | 关键配置 / 源码 | 优先级 | 复杂度 | 可延后 |
|---|---|---|---|---|---|
| VT 解析与屏幕模型 | VT220 + 扩展，alt-screen、鼠标追踪 | `@xterm/xterm ^6.0.0`（`tabby-terminal/package.json:29`） | P0 | 很高 | 否 |
| GPU 渲染 | 默认 WebGL 前端 | `terminal.frontend: 'xterm-webgl'`（`config.ts:19`），`frontends/webglSupport.ts:30` 有 disableGPU 回退 | P0 | 高 | 否 |
| 真彩色 / 主题化 16 色 | 配色方案、亮暗切换 | `terminal.colorScheme` / `lightColorScheme` / `colorSchemeMode`（`config.ts:43-51`、`configDefaults.yaml:22`） | P0 | 中 | 否 |
| 搜索 | 正则/全词/大小写 | `@xterm/addon-search`、`config.ts:53-58` | P1 | 低 | 可 |
| Scrollback | 默认 25000 行 | `config.ts:60` | P0 | 低 | 否 |
| 字体与排版 | 字重、行距、fallback 字体、连字 | `config.ts:20-24,29` | P0 | 中 | 部分 |
| 输入法 / 组合输入 | IME 预编辑、`altIsMeta` | `config.ts:41` | P0 | 中 | 否 |
| 选中与复制 | 复制为 HTML、选中即复制、右键菜单/直接粘贴、中键粘贴 | `copyAsHTML`/`copyOnSelect`/`rightClick`/`pasteOnMiddleClick`（`config.ts:36-39`；Windows 默认 `copyOnSelect:true`，`config.ts:127-129`） | P0 | 中 | 否 |
| 粘贴安全 | bracketed paste、多行确认、平台换行归一 | `baseTerminalTab.component.ts:545-592` | P0 | 低 | 否 |
| Bell | 关/视觉/声音 + 任务栏闪烁 | `config.ts:25-26`、`baseTerminalTab.component.ts:446-454` | P1 | 低 | 可 |
| 分屏 | 四向嵌套分屏、聚焦广播、最大化单 pane | `SplitTabComponent`（`splitTab.component.ts:199-200`） | P0 | 高 | 否 |
| 标签 | 四边标签栏、flex 标签、MRU 循环、pin、拖动重排、复制标签 | `configDefaults.yaml:11-13`、`tabby-core/src/hotkeys.ts` | P0 | 中 | 否 |

### 1.2 会话与传输

| 功能 | 用户可见行为 | 关键配置 / 源码 | 优先级 | 复杂度 | 可延后 |
|---|---|---|---|---|---|
| 本地 PTY | 多 shell、cwd 继承、退出行为 | `tabby-local`；`terminal.useConPTY`（`local/config.ts`） | P0 | 中 | 否 |
| Shell 探测 | CMD/Clink、PowerShell/PS Core、WSL、Git-Bash、Cygwin32/64、MSYS2、Cmder、VS DevTools、POSIX | `tabby-electron/src/index.ts:60-72` 12 个 `ShellProvider` | P0(win) | 中 | 分平台 |
| SSH | 连接管理器、jump host、agent 转发（含 Pageant）、X11、端口转发（L/R/Dynamic）、SOCKS/HTTP 代理、proxyCommand、算法白名单、会话复用、known_hosts 校验 | `tabby-ssh/src/api/interfaces.ts:16-58`、`algorithms.ts`、`services/sshMultiplexer.service.ts` | P0 | 中（russh） | 否 |
| SFTP | 侧栏文件面板、右键菜单、编辑器联动 | `components/sftpPanel.component.ts`、`sftpContextMenu.ts` | P1 | 中 | 可 |
| 串口 | 波特率 110–1.5M、5–8 数据位、奇偶校验、RTS/CTS、hex 输入输出、readline 本地回显、换行转换、慢速发送 | `tabby-serial/src/api.ts:27-38`、`middleware/streamProcessing.ts:11-12` | P2 | 低 | 是 |
| Telnet | 连接管理器 + 会话 | `tabby-telnet`（611 行） | P2 | 低 | 是 |
| Zmodem | 会话内直接收发文件（浏览器选文件） | `features/zmodem.ts:12,329,345` | P2 | 中 | 是 |
| 登录脚本 | expect/send、正则、可选步骤 | `middleware/loginScriptProcessing.ts:5-10`，SSH/串口共用 | P1 | 低 | 可 |
| 会话恢复 | 重启后恢复标签与 profile；能拿到旧 PTY 时直接接管进程 | `api/tabRecovery.ts`、`local/session.ts:62-64` | P0 | 中 | 否 |

### 1.3 窗口、设置与系统集成

| 功能 | 用户可见行为 | 关键配置 / 源码 | 优先级 | 复杂度 | 可延后 |
|---|---|---|---|---|---|
| 设置 UI | 分类设置页、快捷键表、profiles、vault、config sync | `tabby-settings`（2066 行，5 个 tabs） | P0 | 中 | 否 |
| 快捷键 | 多 chord 序列、按平台默认表、冲突检测 | `services/hotkeys.service.ts:263-304` | P0 | 中 | 否 |
| 主题 | CSS 主题、透明度、vibrancy、窗口边框、注入自定义 CSS | `configDefaults.yaml:14-22` | P1 | 中 | 部分 |
| Quake / dock | 侧边停靠窗口、全局热键呼出、失焦隐藏、置顶、指定屏幕与占比 | `configDefaults.yaml:4-10`、`docking.service.ts` | P1 | 中 | 可 |
| 便携模式 | exe 旁建 `data` 目录即整体便携 | `app/lib/portable.ts:7-10` | P2 | 低 | 是 |
| 配置 | YAML `config.yaml` + `.backup`，`TABBY_CONFIG_DIRECTORY` 可改 | `app/lib/config.ts:7,30-32`、`app/lib/index.ts:12` | P0 | 低 | 否 |
| 加密 vault | 整份 config 加密（aes-256-cbc + pbkdf2），启动要口令 | `services/vault.service.ts:16,60,69-80`、`config.service.ts:468-543` | P1 | 低 | 可 |
| 密钥存储 | 密码/key 交给系统钥匙串（keytar） | `app/package.json`（`keytar ^7`）、`ssh/services/passwordStorage.service.ts` | P0 | 低 | 否 |
| 自动更新 | 检查更新并提示；Linux 与 portable 不启用 | `configDefaults.yaml:46`、`electron/src/services/updater.service.ts:23-26` | P2 | 中 | 是 |
| 遥测 | 默认开启（`enableAnalytics: true`）+ Sentry | `configDefaults.yaml:43`、`app/lib/sentry.ts` | P2 | 低 | 是（建议默认关） |
| i18n | 24 个 locale，gettext/Crowdin | `locale/*.po`（24 个） | P1 | 中 | 可 |
| CLI 与 URL scheme | `tabby open <dir>`、`--profile`、`--recent`、quick-connect、`tabby://` | `app/lib/cli.ts:30-73`、`app/lib/index.ts:38-44` | P1 | 低 | 可 |
| Web 版 | 浏览器里跑 SSH（服务端网关） | `tabby-web/src/platform.ts`（只做 `PlatformService` 适配，PTY/SSH 在后端） | P2 | 很高 | 是 |

---

## 2. 插件机制与 API 面

### 2.1 加载与分发（决定"能不能兼容"）

插件就是 npm 包，`package.json` 带 `tabby-plugin` keyword 才会被加载（`HACKING.md:94-100`）；默认导出必须是一个 `NgModule` 类，被注入为根模块的依赖。来源三处：源码 checkout（dev）、用户插件目录、`TABBY_PLUGINS` 环境变量（`HACKING.md:94`、`app/src/plugins.ts:82-83`）。市场不是自建索引：直接打 npm registry 的 `keywords:tabby-plugin` 与 `keywords:terminus-plugin` 搜索接口（`tabby-plugin-manager/src/services/pluginManager.service.ts:33-45`），装包前用 `/^(tabby|terminus)-[a-zA-Z0-9._-]+$/` 正则和黑名单双重拦截（同上 `:97`、`app/src/pluginBlacklist.ts`）。内置 13 个包全部标 `tabby-builtin-plugin`，运行时按 `builtin-plugins` 目录 + `NODE_PATH` 注入（`app/src/plugins.ts`）。

**结论**：这套机制 = "npm 包 + Angular 模块 + Node 运行时"。三者中任何一个不在，插件就跑不起来。Rabby 不该假装兼容（地图已把这条列进 Out of scope），应做**能力对齐**：把 §2.2 的扩展点翻译成 Rust/WASM 的等价 hook。

### 2.2 扩展点清单（上游真实 API 面）

| 扩展点 | 归属包 | 用途 | Rabby 是否需要等价物 |
|---|---|---|---|
| `ConfigProvider` | core | 贡献默认配置命名空间 | 是（配置分片注册） |
| `ProfileProvider` / `Connectable` / `QuickConnect` | core | 贡献 profile 类型 | 是（会话类型注册） |
| `ToolbarButtonProvider` | core | 工具栏按钮 / 子菜单 | 是 |
| `HotkeyProvider` | core | 声明可绑定快捷键 | 是 |
| `TabContextMenuItemProvider` | core | 标签右键菜单 | 是 |
| `TabRecoveryProvider` | core | 会话恢复 token | 是 |
| `Theme` | core | 整份 CSS 主题 | 是（CSS/样式 token） |
| `TerminalColorSchemeProvider` | terminal | 配色方案 | 是 |
| `CLIHandler` | core | 处理命令行参数 | 是 |
| `CommandProvider` | core | 命令面板条目 | 是 |
| `FileProvider` | core | 虚拟文件系统（vault 用） | 视情况 |
| `PlatformService` / `HostApp` / `HostWindow` | core | 平台抽象 | 否（内部 trait 即可） |
| `DockingService` / `UpdaterService` | core | Quake / 更新 | 否 |
| `TerminalDecorator` | terminal | 新终端挂 hook（Zmodem、拖放路径、调试） | 是（关键） |
| `SessionMiddleware` | terminal | 会话字节流中间件（可 push/unshift/replace） | 是（关键） |
| `ShellProvider` | local | 贡献 shell 探测 | 是 |
| `PTYInterface` / `UACService` | local | PTY 后端 / 提权 | 否 |
| `SettingsTabProvider` | settings | 设置页 | 是 |
| `SFTPContextMenuItemProvider` / `SSHProfileImporter` / `AutoPrivateKeyLocator` | ssh | SFTP 菜单、导入 `~/.ssh/config`、找私钥 | 部分 |

合计 31 个 `abstract class`（去重计数，core/terminal/settings/local/ssh 的 api 目录；含 4 个 `FileTransfer` 家族与 2 个 PTY 抽象）。

### 2.3 扩展点背后还有三条"没写在 API 里"的契约

1. **中间件链顺序**：`SessionMiddlewareStack` 的 `push`/`unshift` 决定谁先看到字节（`api/middleware.ts`）；Zmodem 用 `unshift` 抢在 OSC 处理之前（`features/zmodem.ts:345`）。顺序错了功能就静默失效——这是插件作者最容易踩、文档里最不显眼的一条。
2. **Decorator 的 attach/detach 生命周期**：`TerminalDecorator.subscribeUntilDetached()` 负责在标签销毁时取消订阅（`api/decorator.ts`）。第三方插件泄漏订阅的常见原因就是绕过了它。
3. **配置命名空间是"隐式 ABI"**：插件靠 `config.store.terminal.xxx` 这类点路径读写，字段名一旦变，插件静默读 undefined。

---

## 3. 不看源码会漏掉的隐性行为

这些是 README 一句话带过、但用户每天都在用的东西；每条都是 v1 的验收项。

| # | 行为 | 实际实现 | 证据 |
|---|---|---|---|
| 1 | **进度检测**：输出里出现 `NN%` 就在标签上画进度条 | 正则 `(^|[^\d])(\d+(\.\d+)?)%([^\d]|$)`；只在非 alt-screen 生效；要求 `>0 && <=100`（0% 不显示）；下一次不匹配就清空 | `baseTerminalTab.component.ts:530-540` |
| 2 | **8MB 预缓冲**：还没聚焦过的标签先缓存输出 | 上限 8MiB，超出丢最旧块并插入一行 `[... earlier output dropped ...]` 提示 | `session.ts:8,48,75` |
| 3 | **粘贴三件套**：bracketed paste + 多行确认 + 平台换行归一 | Windows 上 `CRLF→CR`、其它平台 `LF→CR`；末尾单换行会被裁掉；可配"换行替换为空格" | `baseTerminalTab:545-560,591-592` |
| 4 | **复制默认带 HTML** | `copyAsHTML: true`，终端富文本粘贴到浏览器/文档保留颜色 | `config.ts:39` |
| 5 | **Unicode 11 宽度**：emoji、双宽 CJK、ZWJ 序列 | `Unicode11Addon` 无条件加载 | `frontends/xtermFrontend.ts:207` |
| 6 | **Sixel 图形** | `ImageAddon` 由 `sixel` 开关控制，默认开 | `xtermFrontend.ts:210-211`、`config.ts:62` |
| 7 | **连字** | `LigaturesAddon` 按需加载，**Web 平台显式禁用** | `xtermFrontend.ts:748-750` |
| 8 | **OSC 处理**：cwd 上报（供新标签继承目录）、程序请求写剪贴板 | `OSCProcessor` 解析 `ESC ]`，跨 chunk 缓冲 | `middleware/oscProcessing.ts`、`session.ts:26-30` |
| 9 | **UTF-8 分块保护** | 保证输出 chunk 不切断多字节字符 | `middleware/utf8Splitter.ts` |
| 10 | **Backspace 编码可配** | ctrl-h / ctrl-? / delete / backspace 四种 | `middleware/inputProcessing.ts` |
| 11 | **Bell 三态** | off/visual/audible + 任务栏闪烁 | `baseTerminalTab:446-454` |
| 12 | **PTY re-attach 式恢复** | 恢复标签时若拿到旧 PTY 句柄，直接接管原进程（不是重开 shell） | `local/session.ts:62-64`、`local/api.ts:46`、`electron/src/pty.ts:25` |
| 13 | **Windows 环境变量重建** | 从注册表 HKLM/HKCU/Volatile 重建并展开 `%VAR%`，Path 追加而非覆盖；开关 `terminal.windowsRefreshEnvironment` | `local/environment.ts:108-126`、`local/config.ts` |
| 14 | **ConPTY 双路径** | 优先 bundled ConPTY dll，失败回退系统内置并打日志 | `local/session.ts:118-131` |
| 15 | **Clink**：Windows 上给 CMD 补 tab 补全 | 默认 profile 就是 `local:cmd-clink` | `local/config.ts:28`、`HACKING.md:63` |
| 16 | **WSL 细节** | 读注册表 `Lxss` 枚举发行版；WSL2 文件根走 `\\wsl$\<distro>`；`--cd ~` 要 build 门槛 | `electron/src/shells/wsl.ts:53-69` |
| 17 | **便携模式** | exe 同级存在 `data` 目录就把 userData 指过去 | `app/lib/portable.ts:7-10` |
| 18 | **配置双写** | 每次保存同时写 `config.yaml` 与 `config.yaml.backup` | `app/lib/config.ts:30-32` |

另外两条不算隐性、但容易被低估：**多 chord 快捷键**（`hotkeys.service.ts:263-304` 按序列长度排序匹配）和**广播输入**（一个输入打到所有 pane/所有标签，`services/multifocus.service.ts`）。

---

## 4. 三类归属：哪些是产品、哪些是适配、哪些是生态

- **独立产品能力（Rabby 必须自己做）**：VT 内核与渲染、标签/分屏/窗口、profile 与会话管理、设置与快捷键、主题与配色、会话恢复、搜索、粘贴/复制语义。
- **平台适配（薄，但平台差异大）**：PTY（ConPTY/posix_openpt）、shell 探测 12 种、全局热键、托盘/菜单/Touch Bar、钥匙串、自动更新、文件对话框、UAC。上游把这一层几乎全放在 `tabby-electron`（3374 行），是"Electron 税"最重的地方；Rust 端对应 `winit/wry` + 平台 crate，行数会显著下降，但 Windows 那块（注册表、WSL、Clink、ConPTY）搬不走。
- **生态（不建议 v1 承担）**：插件市场、主题商店、社区配色（`tabby-community-color-schemes`）、`mcp-server` 之类第三方集成。README 列了 15 个社区插件 + 7 个主题，没有任何一个被内置——说明上游自己也把这块留给社区。

---

## 5. 对 Rabby 的推荐

1. **v1 按地图假设走，但把 §3 的 18 条全部写成验收用例**（每条一个集成测试）。这是"像不像 Tabby"的实际判据。
2. **SSH 直接上 russh，并对齐上游的算法白名单与 jump host 语义**（`tabby-ssh/src/algorithms.ts`）。这是唯一能"抄行为而不抄代码"的地方，因为底层库相同。
3. **终端内核选型优先看"是否已经有 Unicode 宽度表 + sixel + 连字"**，而不是看渲染速度。xterm.js 6 的宽度策略（Unicode 11）和 sixel 是隐性兼容点，自己补的代价远高于换库。
4. **插件机制 v1 只做三条**：配置分片、会话类型注册、终端中间件/装饰器 hook。其余（工具栏按钮、右键菜单、设置页）等有真实需求再说。理由：31 个扩展点里，只有这三条是"不做就没法长"的。
5. **迁移器优先读 `config.yaml`**（YAML，结构见 `tabby-core/src/configDefaults.yaml`），能一次带走 profile、SSH 主机、配色、快捷键；vault 加密的 config 需要用户口令，建议做成可选项而不是默认能力。
6. **迁移器只做"能验证的映射"**。Tabby 的 profile 是 `type + options` 的松散结构（`tabby-core/src/api/profileProvider.ts:8-24`），每个 provider 自己解释 `options`；迁移时按 `type` 分派到 Rabby 的会话类型，**未知字段原样写进迁移日志而不是静默丢弃**——否则用户会看到"profile 还在，但连接参数少了一半"。SSH 部分优先直接读 `~/.ssh/config`（上游本身就有 `SSHProfileImporter`，`tabby-electron/src/sshImporters.ts`），比解析 Tabby 的配置更可靠；
7. **遥测默认关**。上游 `enableAnalytics: true` 是历史包袱，不是产品决策。

---

## 6. 风险与未决

| 风险 | 说明 | 缓解 |
|---|---|---|
| 上游漂移快 | nightly 每天发（`1.0.231-nightly.0`），本盘点绑定 `467ce8e` | 把本文件当快照；对齐机制另开票 |
| 隐性行为漏项 | §3 只是"读到的"，xterm.js 行为面比 README 大一个量级 | v1 验收改为"逐条对标真实 Tabby 截图/录屏" |
| russh 版本锁定 | 上游用 `0.1.38`（0.x，API 不稳） | Rabby 锁自己验证过的版本，不追上游 |
| 插件生态真空 | 不兼容 Tabby 插件 = 迁移用户失去扩展 | v1 用"内置能力 + 脚本化"过渡，市场延后到 v1.x |
| PTY re-attach | 上游也只是"拿得到就恢复"，跨平台差异大 | 降为 P1，v1 只保证"重开同名会话" |
| 未决 | Web 网关形态、移动端、签名分发、i18n 流程 | 均在 map 的 Not yet specified，本票不越权决定 |

---

## 7. 参考

- 仓库：<https://github.com/Eugeny/tabby>（本文件对应 `467ce8e`）
- 功能清单：`README.md:34-48`（总览）、`:75-84`（终端）、`:92-96`（SSH）、`:100-106`（串口）、`:112`（便携）、`:118-133`（插件生态）
- 插件开发：`HACKING.md:56-73`（包布局）、`:92-147`（加载规则、NgModule、provider 示例）
- API 文档站：<https://docs.tabby.sh/>（typedoc 生成，入口 `typedoc.mjs`）
