# Rabby 扩展机制与安全边界（草案 v0.1）

> 状态：14 票「扩展机制与安全边界」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A，本文作为 v1 扩展/安全基线。
> 输入：06（扩展运行时研究）、02（GPUI Kit/gpui-shell）、12（UI 架构）、13（密钥/配置）、07（v1 范围）。
> 一句话：runtime 以 gpui-shell（JS + 声明式 UI + capability）为主、Wasmtime 为无 UI 重活兜底；Rabby 自建 rabby-ext-api ABI 与权限产品；v1 不做第三方运行时，v1.x 在 gpui-shell 稳定后开侧载与签名分发。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A gpui-shell 主运行时 + Wasmtime 无 UI 重活；Q2=A v1 不做第三方运行时，只做 ABI/权限设计 + 宿主内置；Q3=A manifest + 默认拒绝 + 三态授权 + scope + 升级重问 + 审计；Q4=A v1.x 侧载 + Ed25519/minisign 签名，市场后置；Q5=A 六组 API + contribution registry；Q6=A 进程内 + 看门狗/限额，不允许 native/任意进程；Q7=A 不兼容 Angular 插件，常见插件映射为宿主内置/声明式规则 + 迁移指南。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | 运行时与 ABI | A. gpui-shell 为主（JS/TS + 声明式 UI + capability manifest），Wasmtime/Component Model 只做无 UI 重活；Rabby 自建 rabby-ext-api ABI，gpui-shell 是实现细节，不外泄其类型 | B. Wasmtime 为主：UI 表达力不足；C. 自研 JS runtime：重复 QuickJS/声明式 UI/沙箱，成本高 |
| Q2 | v1 范围 | A. v1 不做第三方扩展运行时；只做 ABI/权限/扩展点设计 + 常见 Tabby 插件对应的宿主内置能力；v1.x 在 gpui-shell 稳定后开 JS 侧载，再上 Wasmtime 重活 | B. v1 就开 JS 侧载（无市场）：更早的生态，但 gpui-shell 仍是 M0/preview；C. 永不做第三方扩展：丢 Tabby 生态 |
| Q3 | 权限模型 | A. manifest 声明 + 默认拒绝 + 请求/授予/拒绝三态 + 能力 scope + 升级新增能力重新询问 + 审计日志 + 可撤销；宿主权限页展示 | B. 全有或全无：用户无法做风险取舍；C. 不做权限：等于回到 Tabby 的全权限 Node 插件 |
| Q4 | 签名与分发 | A. v1.x 先本地侧载 + manifest/包签名（Ed25519/minisign）+ 哈希/发布者校验；市场与人工审核再后一阶段；运行时不从 npm 拉任意依赖 | B. v1.x 直接上市场：审核/托管/撤销体系未就绪；C. 不签名：供应链风险不可接受 |
| Q5 | API 面 | A. 六组：ui（commands/keybindings/menus/settings/panels/notifications）、terminal（read/write/subscribe）、sessions（list/create）、config（命名空间）、io（fs/process/net scope）、secrets（handle）；自建 contribution registry | B. 照搬 Tabby 31 个 Angular 扩展点：没有跨语言 ABI；C. 只做 commands：生态表达力不足 |
| Q6 | 隔离边界 | A. 进程内 QuickJS/WASM + 能力门禁 + 看门狗/限额；不允许 native dylib、不允许任意进程；OOM/原生崩溃不隔离，v2 再评估子进程 | B. v1 就子进程隔离：体验/性能/通信成本高；C. 允许 native dylib：等于放弃沙箱 |
| Q7 | Tabby 插件迁移 | A. 不兼容 Angular 插件；常见插件映射为宿主内置或声明式规则 + 迁移指南（docker→session provider、title-control→title rules、quick-cmds→command+terminal.write、save-output→output tap、sync-config→宿主内置、sftp-tab→宿主内置、highlight→highlight rules、mcp-server→宿主内置） | B. 做 Angular 兼容层：等于内嵌 Node/Electron；C. 完全不管旧插件：用户迁移成本高 |

## 1. 运行时分工

- JS/TS（gpui-shell）：
  - 脚本描述 UI 树，宿主每帧 materialize 成 GPUI 元素；脚本不直接渲染、不持有 DOM。
  - 适合：命令、面板、菜单、设置页、终端输出 tap、轻量业务逻辑、HTTP 请求、热重载开发。
  - 能力：capabilities 默认空集；fs read/write roots、process execute、net hosts/HTTP methods/paths、storage、clipboard、env 由宿主授予。
- Wasmtime/Component Model：
  - 适合：无 UI 的重计算、格式解析、协议处理；WASI 默认无网络/文件，host functions 显式授予。
  - 不适合：逐帧 UI（canonical ABI 成本），06 已有结论。
- 不做：Node/Electron、Angular NgModule 兼容、native dylib、任意 npm 运行时安装。
- v1：只做 ABI/权限设计 + 宿主内置能力；v1.x 上 JS 侧载；Wasmtime 重活再后一阶段。

## 2. rabby-ext-api（自有 ABI）

    Manifest {
      id, name, version, api_version, entry,
      publisher, description, homepage, icon,
      capabilities: CapabilityRequest[],
      contributions: Contribution[],
      signature?: ...
    }

- api_version 用 semver；宿主声明支持范围；breaking change 升 major；旧版本保留兼容窗口。
- 包格式：.rabbyext（zip/tar）含 manifest.json、入口 JS/WASM、资源、签名；开发期可用目录。
- contribution registry（gpui-shell 目前缺，Rabby 自建）：
  - commands（id/title/icon/keybinding 默认值）
  - menus（位置/条件/命令）
  - keybindings（默认 chord，冲突检测）
  - settings pages（schema + 字段）
  - panels（dock/side panel；v1.x）
  - terminal hooks（output tap、title/cwd 订阅、middleware）
  - session providers（v1.x，高风险）
- UI：声明式树 + Kit 组件/theme tokens；扩展不能直接创建 GPUI Element；不支持自定义绘制（终端/图表等由宿主组件）。
- 版本协商：manifest api_version + host 版本；不兼容拒绝加载并给出可操作错误；插件声明 shell/kit 兼容范围。

## 3. 权限矩阵

| capability | 作用 | 默认 | 风险 | 备注 |
|---|---|---|---|---|
| ui.commands | 注册命令 | 允许（低风险） | 低 | 命令执行仍需用户触发 |
| ui.keybindings | 注册快捷键 | 需授予 | 中 | 可与宿主冲突；冲突检测 |
| ui.menus | 菜单项 | 需授予 | 低 | 位置/条件 |
| ui.settings | 扩展设置页 | 允许 | 低 | 命名空间隔离 |
| ui.panels | 侧栏/面板 | 需授予 | 中 | 可展示内容 |
| ui.notifications | toast/通知 | 需授予 | 低 | 频率限制 |
| terminal.read | 读输出/标题/cwd | 需授予 | 高 | 可能看到密码/敏感输出；按会话 scope |
| terminal.write | 写输入/注入命令 | 需授予 | 高 | 可执行任意命令；默认拒绝，强提示 |
| terminal.subscribe | 标题/cwd/进度事件 | 需授予 | 中 | 与 read 分开 |
| sessions.list | 枚举会话 | 需授予 | 中 | 元数据 |
| sessions.create | 新建 SSH/本地会话 | 需授予 | 高 | 网络/凭据 |
| config.namespace | 扩展自己的配置 | 允许 | 低 | 只能读写 extensions.<id> |
| fs.read | 读目录/文件 | 需授予（roots） | 高 | cap-std 根目录；拒绝越界 |
| fs.write | 写目录/文件 | 需授予（roots） | 高 | 同上 |
| process.execute | 执行命令 | 需授予（allowlist 或 unrestricted） | 极高 | unrestricted 最高级警告 |
| net.hosts | TCP/WebSocket 主机 | 需授予（hosts） | 高 | 默认无网络 |
| net.http | HTTP scheme/host/port/method/path | 需授予 | 中高 | 与 net.hosts 分开 |
| secrets.handle | 用密钥句柄做认证 | 需授予 | 高 | 不返回原文；host 代为使用 |
| clipboard.read/write | 剪贴板 | 需授予 | 中 | 敏感内容 |
| env | 额外环境变量 | 需授予 | 中 | 默认只给安全白名单 |
| storage | localStorage 类 | 允许（配额） | 低 | 进程内/扩展命名空间 |

- 三态：requested / granted / denied；首次运行权限页；宿主配置持久化；升级新增能力必须重新询问；可随时撤销/禁用。
- 审计：记录扩展 id、能力、时间、结果（不记录 secret/终端内容）；权限页可看。
- 高风险组合（terminal.read + net.hosts / terminal.write + process.execute）额外提示；默认拒绝。
- secrets 只给 handle：扩展请求“用这个 secret 做 SSH/HTTP 认证”，host 内部取 keychain/vault；不把明文交给脚本。需要明文的自定义逻辑 v1.x 再评估，并作为最高风险能力。

## 4. 签名、分发、供应链

- 包签名：Ed25519 或 minisign；manifest 与包哈希；publisher key；安装时校验。
- v1.x 阶段：本地侧载（开发/高级用户）+ 签名校验；市场/人工审核/撤销后一阶段。
- 更新：签名更新包；能力 diff 必须重新授权；可回滚；撤销列表。
- 运行时不从 npm 拉任意依赖；扩展自带 bundle；禁止 postinstall 类行为。
- 审核（市场阶段）：来源/权限/网络域名/文件根/命令 allowlist；固定版本；发布者身份；撤销。
- 威胁模型：恶意扩展（偷终端输出、注入命令、外传密钥）、供应链投毒、权限升级、签名密钥泄露、OOM/崩溃。缓解：默认拒绝、scope、handle secret、签名、审计、看门狗、限额、撤销、子进程隔离留 v2。

## 5. 宿主内置 vs 扩展

- 内置（权限过高或不值得脚本化）：sync-config（网络+密钥）、sftp-tab（SSH/SFTP）、mcp-server（本地 HTTP+终端）、docker（socket/进程）、系统级会话 provider。
- 扩展/声明式：quick-cmds、save-output、title-control（规则）、highlight（规则）、自定义命令/面板/设置。
- Tabby 插件映射：
  - docker → 宿主 session provider + 面板（内置或 Wasmtime）
  - title-control → 声明式 title rules（宿主 hook，无代码）
  - quick-cmds → JS command + terminal.write
  - save-output → JS output tap + fs.write（限根）
  - sync-config → 宿主内置
  - sftp-tab → 宿主内置（SFTP v1.x）
  - highlight → 声明式 highlight rules
  - mcp-server → 宿主内置/系统扩展
- 迁移指南：说明“无 Angular 兼容”，给出每个常见插件的替代；扩展作者文档 + 示例 + check/types 工具（gpui-shell CLI）。

## 6. 与 gpui-shell 的集成

- 宿主：Rabby AppShell/Workspace 提供 contribution registry、权限页、设置页、命令面板、通知、终端/会话/配置/密钥 host functions。
- gpui-shell：ShellRuntime::load / ShellRoot；manifest/capabilities；QuickJS 沙箱；热重载；声明式 UI materialize。
- 缺口（06/官方 README）：gpui-shell 仍是 M0/preview；授权产品（三态/权限页/升级重问）与 contribution registry 要 Rabby 自建；API 可能 minor 变更。
- Wasmtime adapter：同一 rabby-ext-api 的 host functions，WASI 默认无网络/文件；用于无 UI 重活。
- 不把 gpui-shell 类型写进 Rabby 扩展 ABI；适配层隔离，未来可替换。

## 7. 测试与验收

- manifest 校验：缺字段/未知字段/版本不兼容/重复 id/非法路径。
- 权限：默认拒绝；scope 越界；升级新增能力重问；撤销；审计；高风险组合提示。
- 沙箱：fs 越界、网络未授权、进程未授权、env 未授权、clipboard、secret 不可读原文；看门狗超时；内存限额。
- 签名：篡改包/manifest/哈希；未知发布者；撤销；更新能力 diff。
- 终端：terminal.read/write 的 scope 与会话隔离；输出 tap 不丢字节；写输入审计。
- 稳定性：扩展异常不崩宿主；死循环可中断；OOM/原生崩溃已知不隔离，文档写明。
- 供应链：依赖审计；无运行时 npm 安装；包内容清单。

## 8. ADR 草稿要点

- ADR-0008：gpui-shell 主运行时 + Wasmtime 兜底；Rabby 自有 rabby-ext-api；v1 不做第三方运行时。
- ADR-0009：默认拒绝 + scope + 三态授权 + 升级重问 + 审计；secret 只给 handle。
- ADR-0010：v1.x 侧载 + 签名，市场后置；不做 Angular 兼容。

## 9. 证据

- 06：.scratch/rust-rewrite/research/06-extension-runtime.md
- 02：.scratch/rust-rewrite/research/02-gpui-kit-nsh-capability.md
- 12：.scratch/rust-rewrite/specs/ui-architecture.md
- 13：.scratch/rust-rewrite/specs/config-secrets-migration.md
- gpui-shell：crates/shell/README.md、crates/shell/src/capability.rs、crates/shell/src/plugin.rs、docs/gpui-shell.md
