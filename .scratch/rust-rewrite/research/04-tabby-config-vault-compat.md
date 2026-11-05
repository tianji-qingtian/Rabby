# Tabby 配置、密钥与会话兼容面（research 04）

**票号** 04 · **上游** Eugeny/tabby @ `467ce8e`（2026-10-06，commit "xterm 6"）· **来源** 只读源码（行号对应该 commit，路径相对仓库根）、README、本机实机目录（macOS，`config.yaml` 顶层 `version: 8`）、OWASP 与 Electron 官方文档。研究只做读取，未改动任何 Tabby 数据。

---

## 0. 决策摘要

- **迁移面只有三处真源，没有隐藏的数据库。** 一份 `config.yaml`（含内嵌 vault blob）+ `userData/plugins/node_modules`（插件即代码，不可迁移）+ Electron 的 localStorage（LevelDB）。不存在 SQLite、索引文件或独立密钥库；"导入 Tabby"本质是"解析一份 YAML + 一次性解密旧 vault + 重写成本项目格式"。见 `app/lib/config.ts:7,30-32`、`app/lib/index.ts:12`、`tabby-core/src/services/config.service.ts:228-251`。

- **config.yaml 是"稀疏覆盖层"，不是完整快照。** `ConfigProxy.__setValue` 在值等于默认值时会 `delete` 该键（`config.service.ts:117-124`），所以缺失键 = 用当前版本 + 当前平台 + 当前已装插件的默认值。**导入器不能把 Tabby 的缺省语义照搬**，必须以 Rabby 默认值兜底，否则会出现"Tabby 里没配、Rabby 里被写成 Tabby 默认值"的静默行为漂移。

- **vault 必须"只读兼容、重加密落地"，不能沿用格式。** Tabby 是 PBKDF2-HMAC-SHA512 / 100 000 次 / 8 字节 salt + AES-256-CBC / 16 字节 IV / **无 MAC**（`vault.service.ts:13-18,69-98`）；secret 以明文 JSON 塞在同一 CBC 密文里。Rabby 侧应迁到 Argon2id + AEAD（XChaCha20-Poly1305 或 AES-256-GCM），旧 vault 只在导入时解密一次。对照 OWASP 现行建议 PBKDF2-HMAC-SHA512 ≥ 220 000 次（[OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)），Tabby 的参数已偏低，且 CBC 无认证标签意味着密文可被篡改而不报错。

- **明文密钥风险集中在 4 个具体位置，导入器应默认"不落明文"。** ① SSH profile 的 `options.password`（`tabby-ssh/src/api/interfaces.ts:21`，只有在用户手改配置或第三方静态导入文件里才出现）；② 登录脚本 `options.scripts[].send`（`tabby-terminal/src/middleware/loginScriptProcessing.ts:5-10`，expect/send 里常直接写密码）；③ `config.yaml.backup`——**每次保存都会写一份同内容副本**（`app/lib/config.ts:30-33`），vault 未开时它就是第二份明文；④ 未启用 vault 时走 keytar（系统钥匙串），一旦启用 vault，secret 会从钥匙串"搬家"进 CBC blob，但**旧钥匙串条目不会被清理**（`passwordStorage.service.ts:13-58`）。

- **会话恢复数据不可迁移。** `localStorage.tabsRecovery` 里是 RecoveryToken 数组，`savedState` 是 xterm.js 序列化器的输出（`scrollback: 1000`，`excludeAltBuffer/excludeModes`），属于 xterm 私有格式（`app.service.ts:104-121`、`tabRecovery.service.ts:22-49`、`xtermFrontend.ts:802-808`）。Rabby 换终端内核后这份状态无法回放。v1 只迁"最近 profile 列表"（`recentProfiles`，取自 localStorage，`profiles.service.ts:205-213`），不承诺标签恢复。

- **known_hosts 在配置里，不在 `~/.ssh/known_hosts`。** Tabby 从不读 OpenSSH known_hosts，而是 `ssh.knownHosts: [{host, port, type, digest}]`，`digest = base64(sha256(公钥 blob))`（`sshKnownHosts.service.ts:4-32`、`ssh.ts:606-619`）——正好等于 OpenSSH `SHA256:...` 冒号后那一段，**可双向映射**，这是迁移里最干净的一项。

- **票面"WinSCP 导入"是错误假设，需修正。** 本基准只有 WinSCP **启动集成**（读注册表定位 `WinSCP.exe`，`tabby-ssh/src/services/ssh.service.ts:22-128`）；`sshImporters.ts` 全文无 WinSCP 解析器。已回查 `v1.0.190` tag 的同一文件，同样只有 `OpenSSHImporter / StaticFileImporter / PrivateKeyLocator` 三个类，说明不是近期删掉的。

---

## 1. 磁盘布局与外部依赖

| 路径（macOS） | 内容 | 迁移相关性 |
|---|---|---|
| `~/Library/Application Support/tabby/config.yaml` | 全部非插件数据；可整份被 vault 加密 | **主输入**。路径 = `TABBY_CONFIG_DIRECTORY` ?? `app.getPath('userData')`（`app/lib/index.ts:12`；Electron 默认 userData 见[官方文档](https://www.electronjs.org/docs/latest/api/app#appgetpathname)：macOS `~/Library/Application Support/<app name>`、Windows `%APPDATA%\<app name>`、Linux `$XDG_CONFIG_HOME` 或 `~/.config`） |
| `config.yaml.backup` | 每次保存的完整副本（同明文/同密文） | 只读兜底；**本身是明文泄露面** |
| `plugins/node_modules/` | 用户插件（npm 包，`tabby-*` / 旧 `terminus-*` 前缀） | 不可迁移（Angular 插件） |
| `ssh-profiles.yaml` | 手工静态导入的 SSH profile 列表（`StaticFileImporter` 读，非 config 的一部分） | 可选迁移，等价于 OpenSSH config |
| `ssh-profiles-cache.json` | OpenSSH config 解析结果 + mtime 缓存 | 忽略（派生数据） |
| `Local Storage/leveldb` | `tabsRecovery`、`recentProfiles`、`profileGroupCollapsed`、`vaultRememberPassphraseFor`、analytics id | 只有最近 profile 值得迁；需解析 LevelDB（Chromium 私有格式） |
| `<exe>/data/`（便携模式） | 整个 userData 被重定向到这里（`app/lib/portable.ts:5-11`；README "Portable"） | 迁移器要把"config 目录"作为参数，不要硬编码 |
| `../terminus/config.yaml` | 更早期产品名，启动时按 mtime 迁移一次（`app/lib/config.ts:8-18`） | 仅历史说明 |

---

## 2. config.yaml 的 schema 与版本

**顶层键**（`tabby-core/src/configDefaults.yaml` 全量 63 行，加各包 `ConfigProvider.defaults`）：`version`、`profiles[]`、`groups[]`、`profileDefaults`、`hotkeys`、`appearance`、`terminal`（core 视觉项 + terminal 内核项 + local 的 `autoOpen/useConPTY/environment` + `profile` 默认 profile id）、`accessibility`、`hacks`、`ssh`、`serial`、`telnet`、`clickableLinks`、`configSync`、`recoverTabs`、`vault`、`encrypted`、`enableAnalytics`、`enableWelcomeTab`、`enableAutomaticUpdates`、`enableExperimentalFeatures`、`hideTray`、`language`、`electronFlags`、`defaultQuickConnectProvider`、`showProfileTree`、`pluginBlacklist`/`commandBlacklist`/`providerBlacklist`/`profileBlacklist`。

**三条关键语义**：

1. **默认值合并顺序**（`config.service.ts:194-207`）：`provider.defaults` ← `platformDefaults[configPlatform]` ← `platformDefaults[platform]`，然后所有 provider 之间 `reduce(configMergeByDefault)`。**后注册的插件可覆盖先注册的默认值**——所以同名配置键的实际含义取决于已装插件集合，导入时不能只按"核心包"理解。
2. **`__nonStructural`**：标在 `env`、`hotkeys.profile`、`profileDefaults`、`colorScheme` 等对象上，表示"整体当作叶子值、不递归合并"（`config.service.ts:24-31,90-102`）。这些键在 Rabby schema 里需要"整体替换"而非"逐键合并"。
3. **版本迁移 0→8**（`config.service.ts:326-466`）：v1 `privateKey`→`privateKeys[]`；v2 `terminal.profiles`→顶层 `profiles`、`recoverTabs` 上浮；v3 `ssh.connections`/`serial.connections`→profiles、jumpHost 从名字改存 profile id；v4 补 `id`；v5 `group` 名字→`groups[]` 的 id（含嵌套 `parentGroupId`）；v6 `ssh.clearServiceMessagesOnConnect`→`profileDefaults.ssh`；v7 删 `configSync.host/token` 默认端点；v8 压缩算法降级为 `['none']`。**导入器应实现一个只读的"按版本补字段"子集（至少 v3→v8），或要求 Tabby ≥ 某版本**。

保存：`yaml.dump(JSON.parse(JSON.stringify(store)))`，`atomically` 写 `.tmp` 再落盘 + 写 `.backup`（`config.service.ts:241-251`、`app/lib/config.ts:30-33`）。

**profile / group / provider 模型**：`Profile { id, type, name, group, options, icon, color, disableDynamicTitle, behaviorOnSessionEnd, weight, isBuiltin, isTemplate }`（`tabby-core/src/api/profileProvider.ts:8-23`）。`id` 形如 `ssh:custom:<slug>:<uuid>`、内置 `local:<shellId>` / `ssh:template`；`type` 必须等于某个 `ProfileProvider.id`，内置四个：`local`、`ssh`、`serial`、`telnet`（docker/k8s 等是外部插件，`type` 值未知 → 只能原样保留或丢弃）。`Group { id, parentGroupId?, name, icon?, color?, defaults: {<providerId>: <options>} }`（`profileProvider.ts:39-48`）支持嵌套与按组默认值。默认值最终合并顺序：基础 `profileDefaults`（代码内）→ `provider.configDefaults` → `config.profileDefaults[type]` → `groups[].defaults[type]`（`profiles.service.ts:405-414`）。

---

## 3. vault：格式、口令、钥匙串

| 项 | 值 | 来源 |
|---|---|---|
| 存放位置 | `config.yaml` 的 `vault` 字段 | `config.service.ts:188,238,321` |
| 外层结构 | `{version: 1, contents: base64, keySalt: hex(8B), iv: hex(16B)}` | `vault.service.ts:20-25,69-84` |
| 内层明文 | `{config?: <整份配置>, secrets: VaultSecret[]}` | `vault.service.ts:40-57` |
| KDF | PBKDF2-HMAC-SHA512，100 000 次，64-bit salt，输出 32B | `vault.service.ts:13-18,59-67` |
| 加密 | `aes-256-cbc` + 随机 128-bit IV，PKCS#7，**无 MAC/无 AAD** | `vault.service.ts:16,69-98` |
| 每次保存 | salt 与 IV 都重新随机 → 每次写盘密文不同 | `vault.service.ts:70-71` |
| 版本校验 | `version !== 1` 直接抛错 | `vault.service.ts:87-89` |
| 口令缓存 | 仅进程内存（模块级变量），到期清空；localStorage 只存"记住多久" | `vault.service.ts:102-104,182-198`、`unlockVaultModal.component.ts:10-32` |
| secret 类型 | `ssh:password`（key `{user, host, port}`）、`ssh:key-passphrase`（key `{hash}`）、`file`（key `{id, description}`，value 为 base64 文件） | `passwordStorage.service.ts:6-7,102-112`、`vault.service.ts:100,310-326` |
| 匹配语义 | `keyMatches` = 提供的每个字段全等；查不到时把 `host` 置空再找一次（"全局默认账号"） | `vault.service.ts:200-213,250-252` |
| 未启用 vault | keytar：service `ssh@<host>[:<port>]`，account = 用户名；私钥口令 service `ssh-private-key:<sha512hex>`，account `user` | `passwordStorage.service.ts:60-100` |
| 整份配置加密 | `encrypted: true` 时，除 `vault`/`encrypted`/`configSync` 外全部塞进 `vault.config` | `config.service.ts:468-559` |

**导入含义**：三种状态都要处理——(a) 无 vault：无 secret 可迁，但要提示"你的密码仍在系统钥匙串"; (b) 有 vault 未加密配置：只需口令解密 secrets，profile 明文可读；(c) `encrypted: true`：**连 profile 都要口令才能读**，导入必须在 UI 里先要口令，且失败时给出"忘记口令 = 无法迁移"的明确结论。另注意私钥口令的 key 是**私钥文件内容的 sha512**，Rabby 只有保持同一 hash 方案才能把旧口令对上具体私钥；否则只能让用户重新输入一次。

---

## 4. SSH：known_hosts、私钥、agent、导入器

- **known_hosts**：仅存在于 `config.yaml` 的 `ssh.knownHosts`，`verifyHostKeys` 默认 `true`（`tabby-ssh/src/config.ts:5-14`）。挑战时若 `digest` 不符则弹窗，接受后写回配置（`hostKeyPromptModal.component.ts:18-36`）。**不读 `~/.ssh/known_hosts`**，也不写回 OpenSSH 文件。digest 算法见 `ssh.ts:612`；`type` 是算法名（如 `ssh-ed25519`）。
- **私钥**：`options.privateKeys: string[]`，元素是路径或 file provider 引用；`ElectronFileProvider` 只接受绝对路径或 `file://` 前缀（`fileProvider.service.ts:33-40`），从 OpenSSH config 导入时会把 `~/` 展开成绝对路径（`sshImporters.ts:275-284`）。连接时支持 `%h`/`%r` 占位符替换（`ssh.ts:212-215`）；私钥可存在 vault 里（`vault://<id>`，经 `VaultFileProvider`）。
- **agent**：`ssh.agentType: 'auto'`、`agentPath`（`config.ts:9-10`）；未显式配 auth 时先试 agent（含 Pageant），无 `privateKeys` 时会枚举 `~/.ssh/id_*` 逐个尝试（`sshImporters.ts:470-489`、`ssh.ts:243-247`）。这部分是运行时行为，不产生可迁移数据。
- **三个导入器**（`sshImporters.ts`）：`OpenSSHImporter`（解析 `~/.ssh/config`，展开 `Include` 并递归、跳过通配 Host、`HostName` 缺省时取别名、`ServerAliveInterval/ConnectTimeout` 秒→毫秒、`ProxyJump` 转成指向 `openssh-config:<sha256(host)>` 的 jumpHost、`LocalForward/RemoteForward/DynamicForward` 转 `forwardedPorts`）；`StaticFileImporter`（`userData/ssh-profiles.yaml`，`id = file-config:<sha256(name)>`）；`PrivateKeyLocator`（只枚举不存储）。

**关键坑**：前两个导入器的产物是 **运行时 builtin profile**（`profiles.ts:66-96` 把它们标成 `isBuiltin: true`），**从不写入 config.yaml**。所以 Rabby 拿不到"Tabby 导入过的 OpenSSH 主机"——必须自己再跑一遍 OpenSSH 解析（或直接复用这些规则）。

---

## 5. 会话恢复的数据结构

`localStorage.tabsRecovery` = `RecoveryToken[]`；保存时机是 tabs 变化/30s 心跳防抖，以及关窗时强制一次（`app.service.ts:104-121,554-556`）。单个 token：

| 字段 | 说明 |
|---|---|
| `type` | `app:local-tab` / `app:ssh-tab` / `app:serial-tab` / `app:telnet-tab` / `app:split-tab` |
| `profile` | **整份 profile 快照（含 options）**，不是引用；恢复时按此重建 ConfigProxy |
| `savedState` | xterm 序列化字符串（scrollback 1000，排除 alt buffer 与 modes） |
| `tabTitle/tabCustomTitle/tabCustomTitleVisible/tabPinned/tabIcon/tabColor/disableDynamicTitle` | 标签外观 |
| `app:split-tab` | `{ratios, orientation, children}` 递归，可任意嵌套（`splitTab.component.ts:109-124`） |

结论：这份数据既在 Chromium LevelDB 里（需自己解析），`savedState` 又是 xterm 私有序列化，**跨内核不可用**；即便解析出 `profile`，其价值也等价于"再导入一次 profile"，不如直接迁 `config.profiles`。

---

## 6. CLI / 环境变量 / 便携模式

- CLI 子命令：`open [dir]`、`run [cmd...]`（Windows 兼容 `/k`）、`profile [name]`、`paste [text]`（`-e` 转义）、`recent [i]`、`quickConnect <providerId> <query>`，全局 `-d/--debug`、`--hidden`（`app/lib/cli.ts:25-98`）。单实例锁 + `second-instance` 转发 argv（`app/lib/index.ts:83-90`），`tabby://` URL scheme（`index.ts:38-45`）。
- 环境变量：`TABBY_CONFIG_DIRECTORY`（改配置目录，`index.ts:12`）、`TABBY_PLUGINS`（额外插件搜索路径，冒号分隔，`plugins.ts:82-84`）、`TABBY_DEV`（开发模式路径）、`PORTABLE_EXECUTABLE_FILE`/`PORTABLE_EXECUTABLE_DIR`（便携版 relaunch 与 shell 集成，`hostApp.service.ts:70-78`）。
- **便携模式判定 = exe 同目录存在 `data/`**，存在则 `app.setPath('userData', <exe>/data)`（`portable.ts:5-11`）。

---

## 7. importer 映射表

| Tabby 数据 | Rabby 目标 | 处理方式 | 备注 |
|---|---|---|---|
| `profiles[]`（local/ssh） | 本机 profiles | **自动 + 审阅** | 字段级翻译；未知 `type`（docker/k8s 等插件）保留原始 JSON 供手动重建 |
| `profiles[].options.privateKeys` | 私钥引用 | 自动 | `file://`→绝对路径；`vault://` 需先解 vault |
| `profiles[].options.password` | secret store | **需确认，默认不导入** | 明文出现在 config 时属泄露 |
| `profiles[].options.scripts[].send` | 登录脚本 | **需确认** | 常含明文密码 |
| `groups[]` / `profileDefaults` | 分组与默认值 | 自动 | 需处理 `__nonStructural` 整体替换语义 |
| `hotkeys.*` | 快捷键 | **需映射表 + 冲突检测** | 字符串格式 `Ctrl-Shift-T`/`⌘-W`，chord 序列为数组；动作 id 与 Rabby 不一定同名 |
| `appearance.theme` | 主题 | **按名字匹配，未命中回退** | 只是字符串 name，指向某个插件的 CSS 主题（`themes.service.ts:195-201`） |
| `appearance.css` | 自定义 CSS | 需确认 | 直接注入 DOM，Rust/GPUI 无等价物 |
| `terminal.*` | 终端设置 | 自动（能映射的） | `frontend`/`font`/`paletteGenerate` 等内核相关项不可映射 |
| `terminal.colorScheme/lightColorScheme` | 配色 | 自动 | 完整对象内嵌在 config（`tabby-terminal/src/config.ts:43-51`），含 16 色 + fg/bg/cursor |
| `terminal.customColorSchemes[]` | 配色库 | 自动 | 同为内嵌对象数组 |
| `ssh.*`（除 knownHosts） | SSH 设置 | 自动 | `agentType/agentPath/winSCPPath/x11Display` 平台相关 |
| `ssh.knownHosts[]` | known_hosts | **自动 + 指纹提示** | 可直接映射；建议顺带导入 OpenSSH `known_hosts` |
| vault secrets | 新 secret store | **逐条审阅导出** | 需要口令；迁移后旧 vault 仍留磁盘，应提示用户自行删除 |
| keytar 条目 | 新 secret store | **不可自动（跨 app ACL）** | 需用户在钥匙串授权或手输 |
| `recentProfiles` | 最近列表 | 可选 | 在 LevelDB 里 |
| `tabsRecovery` | — | **不迁** | 见 §5 |
| `configSync.*`、`enableAnalytics`、analytics id、`electronFlags`、`hacks`、`pluginBlacklist`、`plugin` 配置 | — | **丢弃** | 语义绑定 Electron/插件市场 |
| `plugins/node_modules` | — | **不可迁** | Angular 插件 |

---

## 8. 安全注意（迁移过程本身）

1. **只读原文件、不写回**：任何"就地脱敏"都会破坏用户回滚能力；导入器应复制到内存、解析完立即丢弃。
2. **明文中间态**：解 vault 后在内存里是完整 JSON（含密码）；Rust 侧应使用 `secrecy`/`zeroize` 包装、避免 Debug 打印，且不要落到任何临时文件或日志。
3. **旧 vault 留在原处**：迁移完成后 `config.yaml` 仍在，CBC 密文与 `.backup` 副本等于"旧口令仍然有效"；应显式告知用户如何处理（删除 or 保持）。
4. **口令强度**：8 字节 salt + 100k PBKDF2 可被离线爆破；导入时的口令输入框应提示这一点，并在重加密后建议更换高强度口令。
5. **keytar/钥匙串跨进程读取**：macOS Keychain 条目由 Tabby 创建，另一个 app 读取通常触发用户授权；不要把它设计成"静默读取"，应作为可选的"用户手动授权"步骤。（此条为平台行为推断，未在本机验证，标为中等置信。）

---

## 9. 推荐（给 Rabby 的落地形态）

1. **独立 crate `rabby-import-tabby`**：纯只读模块，输入 config 目录，输出 `ImportPlan`（强类型、可 JSON 序列化），由 UI 逐组确认后应用。三阶段分离：`detect → plan → apply`，可 dry-run。
2. **vault 双实现**：`LegacyTabbyVault`（仅解密，PBKDF2-SHA512 100k + AES-256-CBC，只为一次性导入）+ `RabbyVault`（Argon2id + XChaCha20-Poly1305，secret 级别 AEAD，附带 KDF 参数版本号与迁移钩子）。不要提供"继续使用 Tabby 格式"的开关。
3. **schema 采用"版本化 + 稀疏"与"完整"二选一**：若沿用稀疏覆盖，必须把默认值表随二进制版本固化并在导入时记录 `imported_from: tabby@<version>`；否则调试会非常痛苦。建议 Rabby 自己用**完整规范化**存储（显式写出所有生效值）。
4. **密钥默认进 OS keychain**（Keychain / Credential Manager / secret-service），vault 作为"可选加密容器"而非唯一路径——Tabby 的用户里相当一部分从未启用 vault。
5. **导入报告**必须列出：成功项、被跳过项（含原因）、需要用户重新输入的秘密、以及"哪些行为在 Rabby 里没有等价物"（主题 CSS、`appearance.css`、插件专属配置）。

---

## 10. 置信度与未决

- **高**：config.yaml 结构、vault 加密参数、profile/group 模型、known_hosts 与私钥语义、导入器行为、便携模式与 env 变量（全部有 `文件:行号`）。
- **中**：keytar 条目跨 app 读取的实际授权行为（平台行为，未实测）；`plugins/node_modules` 中存在外部插件自定义 profile type 时其 options 形状（无法穷举）。
- **未决**：Rabby 是否需要一个"读旧 vault"的常驻兼容层（建议不要，只做一次性导入）；是否为 `appearance.css` 提供等价能力（否则这是最容易被用户投诉的迁移缺口）；是否需要解析 LevelDB 取 `recentProfiles`（成本高、收益低，倾向不做）。

---

### 参考

- 源码（`467ce8e`）：`tabby-core/src/services/{config,vault,profiles,tabRecovery}.service.ts`、`tabby-core/src/configDefaults*.yaml`、`tabby-ssh/src/{config.ts,services/*,session/ssh.ts,profiles.ts,recoveryProvider.ts}`、`tabby-electron/src/{sshImporters.ts,services/{platform,hostApp,fileProvider}.service.ts}`、`app/lib/{config,index,portable,cli}.ts`
- 仓库：<https://github.com/Eugeny/tabby/tree/467ce8e>
- Electron `app.getPath`（userData/appData 路径规则）：<https://www.electronjs.org/docs/latest/api/app#appgetpathname>
- OWASP Password Storage Cheat Sheet（PBKDF2-HMAC-SHA512 ≥ 220 000 次等）：<https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html>
