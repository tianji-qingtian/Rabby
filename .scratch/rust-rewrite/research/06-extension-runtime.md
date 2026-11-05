# 06 扩展运行时选型：WASM 组件 vs JS 宿主

- 日期：2026-11-05｜上游：Eugeny/tabby @ `467ce8e`（本地 `/tmp/tabby-recon`）
- 一手来源：Tabby 源码；gpui-kit `docs/gpui-shell.md`（下称 **GS**，`GS §x.y` 对应 main 分支 2026-11-04 快照）与 `website/shell/*.md`；crates.io API；Zed / Extism / wasm3 官方仓库与 Cargo.toml。

## 结论

**主运行时 gpui-shell（QuickJS，进程内 JS + 声明式 UI 描述树 + capability manifest），兜底 Wasmtime + Component Model。** 这不是"JS 好写"的偏好，而是三条硬约束的交集：扩展要能画 UI，而 GPUI 没有跨语言的 retained-mode 组件 ABI；交互边界调用必须便宜；Rabby 的 UI 已押在 GPUI Kit 上，gpui-shell 与它同源，官方文档甚至直说"想要 JavaScript 扩展点的 Rust GPUI 应用走同一条嵌入路径"（`getting-started.md`「Move to a Rust host」）。WASM 组件只在**无 UI**时成立——每次调用都要过 canonical ABI 序列化（component-docs `advanced/canonical-abi.md`），拿它做逐帧 UI 是选错边界，gpui-shell 自己也把 component model 列为已否决（GS §24）。

真正的成本不在选引擎，而在 gpui-shell 自认没做的两块：**授权产品**与**贡献注册表**（GS §18.4）。它们必须由 Rabby 实现，且要做成 Rabby 自有 crate 里的稳定 ABI。

## 1. 基线：Tabby 插件是什么

| 事实 | 证据 |
|---|---|
| 插件 = npm 包，准入条件只有 `tabby-plugin` keyword；default export 是注入根模块的 Angular `NgModule` | HACKING.md:96、:100 |
| 分发 = npm；管理器用 Arborist 就地 `reify` | HACKING.md:147；app/lib/pluginManager.ts:6-9 |
| 扩展点 = Angular DI multi-provider（Toolbar/Config/Hotkey/SettingsTab/Profile/TabContextMenu/TerminalDecorator/TabRecovery…） | tabby-core/src/api/index.ts:1-42；tabby-terminal/src/api/decorator.ts:7-13 |
| 生态规模：npm 上带该关键字的包 **193** 个 | registry.npmjs.org search `total` |

要点是**权限为零、表达力为满**：`tabby-save-output` 直接 `fs.createWriteStream`（save-output/src/decorator.ts:40）；`tabby-docker` 引 `dockerode` 连 socket 并自造 `BaseSession`（docker/src/services/docker.service.ts:3,131）；`tabby-quick-cmds` 直接 `currentTab.sendInput()`（quick-cmds/src/buttonProvider.ts:181）。Tabby 插件是能改 UI、能起进程的 Node 程序，不是配置片段——这否掉了"只做 WASM 命令插件"的窄方案。

## 2. 候选横评

| 维度 | gpui-shell（QuickJS） | Wasmtime + Component Model | Extism | wasm3 | 原生 dylib |
|---|---|---|---|---|---|
| UI 表达力 | **强**：脚本 `render` 出描述树，宿主 materialize；`ScriptPanel` 可作 dock 面板（GS §8、§15） | 弱：需自造 WIT 渲染协议，逐帧跨边界 | 无：导出函数式插件 | 无 | 强，但等于暴露宿主 ABI |
| 沙箱 | 中强：默认空能力 + 语言裁剪 + 限额（GS §19） | **最强**：线性内存 + WASI 能力注入 | 强（复用 wasmtime） | 中（WASI 不全） | **无** |
| 热重载 | **有**：200ms debounce，换对象保留实体/焦点（GS §21.2） | 需自建，状态丢失 | 需自建 | 需自建 | 难（卸载/重入/TLS） |
| 性能 | 443 节点 1.4ms、materialize 0.7ms、未变更帧 **0 次进 VM**（GS §20.3） | 边界贵、计算近原生 | 同左 | 解释执行，慢数倍 | 最快 |
| 工具链/调试 | `check`/`types`/`--print-spec`，**无 DevTools**（GS §21.3） | 重：wasm-tools、cargo-component、DWARF 链 | 中 | 轻 | 中（分平台签名） |
| 成熟度 | **0.9.0，2026-11-02**，自标 preview/experimental | wasmtime 48.0.8（2026-10-29），WASI 0.3 已加 async/stream/future | 1.41.0（2026-10-09） | README 声明 minimal maintenance；绑定停 0.3.1/2021 | — |

原生 dylib 的否决是硬的：Rust 无稳定 ABI（`abi_stable` 这类 crate 即绕行方案，0.11.3 停更于 2023-10），且加载即全权限、崩溃拖垮宿主、每平台都要签名公证。

## 3. gpui-shell 关键事实（主候选）

| 面 | 事实 | 出处 |
|---|---|---|
| 引擎 | `rquickjs` 0.12，唯一引擎（`compile_error!` 兜底）；无 JIT，作者承认热循环与边界调用赢不了 JIT | GS §6.3、§6.5 |
| 渲染 | 每帧完整描述 view，宿主 materialize；快照未变不进 VM（benchmark C 断言 50 次重绘 = 0 次脚本 render）。443 节点 1.4ms，占 120Hz 8.3ms 预算约 17%，且只在失效帧支付 | GS §8、§20.3 |
| 能力 | manifest 七字段；`fs.read/write/execute`、`network.hosts`+`http`(方法/路径)、`storage`、`clipboard`；拒绝信息直接指向要补的 key | GS §18.1 |
| 沙箱 | 不编译 quickjs-libc `std`/`os`、删 `eval`、替换四类 Function 构造器、冻结原型、自写 resolver；2s/5s/10s 看门狗不可被 catch 吞掉；模块 8MiB、子进程 30s | GS §19.1–19.3 |
| 权限归属 | grant 挂在**调用帧** `Policy`，非进程全局——"await 跨越时间，时间作用域的 guard 描述不了更长的权限" | GS §18.3 |
| 嵌入 | `ShellRuntime::new` → `set_capabilities` → `load(&root,window,cx)` 得 `ShellRoot`；多插件用 `PluginManager`（load 需宿主授权回调） | `getting-started.md`；GS §18.4 |
| 缺口 | 无授权产品（granted/denied/prompt、权限页、更新加权限再询问）；无贡献注册表；无 DevTools | GS §18.4 |

## 4. WASM 组件（兜底）与 wasm3

组件模型已可生产但仍在动：WASI 0.3 把 `async func`/`stream`/`future` 加入 canonical ABI。成本模型是决定性的——每次调用都要序列化，适合"一次调用返回一坨结果"的后台任务，不适合每帧 200–800 节点的 UI 描述。先例印证：Zed 用 `wasm32-wasip2` + wasmtime，扩展能力只有 languages/debuggers/themes/snippets/MCP，**没有面板或命令 API**（zed `docs/src/extensions/developing-extensions.md`、`crates/extension_host/Cargo.toml:55-57`）；Extism 1.41.0 底层即 wasmtime 49（`runtime/Cargo.toml:12-24`），定位是 headless 不受信插件。wasm3 出局：作者声明 minimal maintenance，Rust 绑定 2021 年后未动。

## 5. UI 集成点：三条路径

| 路径 | 谁画 | 适用 | 代价 |
|---|---|---|---|
| **A 声明式贡献**（manifest 写 commands/menus/keybindings/settings，处理器是函数） | 宿主 GPUI Kit | 多数 Tabby 插件 | 表达力有上限，但统一、快、易审计 |
| **B 脚本面板**（`ScriptView` 进 dock panel，GS §15） | 扩展描述、宿主 materialize | docker 列表、sftp 树、状态页 | 受渲染协议约束，有 API 漂移风险 |
| **C WebView**（GS §14.8） | 扩展自带 HTML | 移植型复杂 UI | 网络按 `fetch` 计权限；**Linux Wayland 不可用**；Windows 上原生化会盖住 GPUI 内容 |

建议 A 默认、B 白名单、C 不公开。

## 6. Tabby 插件的新形态映射

| 插件 | 现在 | Rabby 形态 | 运行时 | 能力 |
|---|---|---|---|---|
| docker | dockerode + 自造 BaseSession | 会话 provider + 面板 | 宿主内置或 WASM | `process.execute`/socket |
| title-control | 监听标题转义序列改写标题 | 规则表 | 无代码 | 无 |
| quick-cmds | toolbar + 弹窗 + sendInput | 命令贡献 + 参数模板 | A（JS） | `terminal.write` |
| save-output | 订阅输出写文件 | 输出 tap | A/B（JS） | `fs.write`（限目录） |
| sync-config | Gist 网络 + 配置 | **宿主内置** | 宿主 | 网络白名单 + 密钥库 |
| sftp-tab | SSH + 文件树 | 内置，协议可选 WASM | 宿主/WASM | `session.*`、`fs` |
| highlight | TerminalDecorator 改渲染 | 声明式高亮规则 | 无代码（宿主 hook） | 无 |
| mcp-server | 本地 HTTP + 终端 IO | 内置或"系统级扩展" | 宿主优先 | `net.listen`+`terminal.*` |

八行里真正需要脚本运行时的只有 quick-cmds 与 save-output；docker 与 highlight 是宿主 hook，sync-config / sftp-tab / mcp-server 权限太高应当内置。这也是 v1 先只做 A 路径的理由。

## 7. 权限与供应链

能力声明走 manifest 白名单 + 默认拒绝（`Capabilities::default()` 空集且构造走 builder，GS §19.2），但 manifest 只能**申请**不能**自证**（GS 原文："it never approves those requests"）。Rabby 必须补：三态 requested/granted/denied、首次运行权限页、决定存宿主配置、**升级新增能力必须重新询问**——这正是 GS §18.4 承认没做的一号安全项。

签名没有事实标准。现实解是「包摘要 + manifest 签名（Ed25519/minisign）+ 注册表审核」：Zed 就是这样，扩展进 `zed-industries/extensions` 走 PR 审核、固定 commit、校验 `extension.toml` 版本（`publishing/publishing-guide.md`）。**别重蹈上游**：Tabby 用 Arborist 装任意 npm 包，`tabby-plugin-manager/src` 搜不到任何 verify/signature/integrity 逻辑，唯一管控是硬编码名字黑名单（app/src/pluginBlacklist.ts:1-11）。193 个社区包 = 193 个可全权限运行的 Node 程序。

隔离要诚实：JS 与 WASM 都在宿主进程内，看门狗杀得掉死循环，杀不掉 OOM 与原生崩溃；真隔离只能上子进程（VS Code 路线）。v1 建议接受进程内 + 限额。

## 8. 风险与未决

| 风险 | 影响 | 缓解 |
|---|---|---|
| gpui-shell 0.9.0 / preview，API 可能 minor 变更 | 高 | 扩展 ABI 定义在 Rabby 自有 crate，gpui-shell 作实现细节；锁版本；缺口推上游 |
| 无 contribution registry | 高 | 自建 `Contributions`（commands/keybindings/menus/settings/panels）+ manifest 解析 + 热注册 |
| QuickJS 无 JIT，重计算卡 UI 线程 | 中 | 重活外移到 WASM/宿主 Rust，脚本侧只暴露异步 |
| WebView 在 Linux Wayland 不可用 | 中 | 不作为公开扩展能力 |
| 调试只有 `check`/`types`/`--print-spec` | 中 | 结构化日志 + 快照测试兜底 |

未决：① 扩展 ABI 的版本协商（照抄 `shell-version` 还是自建 semver 契约）；② 权限页形态与默认值；③ WASM 兜底是 v1 就做还是先留 trait 口子。

## 9. 交给实施票的最小 API 面

按 §6 的映射倒推，扩展宿主必须暴露的只有六组，多一组都是负担：**terminal**（读输出、写输入、订阅 title/cwd）、**sessions**（枚举/新建 SSH、串口留给内置）、**config**（命名空间读写，带默认值与迁移）、**ui**（命令、菜单、快捷键、设置页、可选的脚本面板）、**io**（fs 限定根、process 白名单、net 主机白名单）、**secrets**（走系统钥匙串，脚本只见句柄）。其余（docker、sftp、mcp）都该是宿主能力，由扩展**调用**而不是**实现**。

**置信度：方向高（JS + 声明式 UI 与 GPUI Kit、Tabby 插件形态三方吻合）；gpui-shell 生产就绪度中（版本 0.9、自标 preview、两处关键能力缺失）。**

## 参考

- Tabby：HACKING.md、app/lib/pluginManager.ts、tabby-core/src/api/、tabby-terminal/src/api/、插件源码（tabby-docker/-save-output/-highlight/-quick-cmds）
- gpui-kit：https://github.com/longbridge/gpui-kit —— `docs/gpui-shell.md`（§6.3/§8/§14.8/§15/§18/§19/§20.3/§21/§24）、`website/shell/getting-started.md`、`website/shell/capabilities.md`
- crates.io：wasmtime 48.0.8、wasmtime-wasi 48.0.8、extism 1.41.0、rquickjs 0.14.0、wasm3 0.3.1、gpui-shell 0.9.0、abi_stable 0.11.3
- Component Model：https://component-model.bytecodealliance.org/（`advanced/canonical-abi.md`）
- Zed：`docs/src/extensions/*`、`crates/extension_host/Cargo.toml`｜Extism：README、`runtime/Cargo.toml`｜wasm3：README 维护声明
