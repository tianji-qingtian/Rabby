# Rabby 配置、密钥与迁移（草案 v0.1）

> 状态：13 票「配置、密钥与迁移」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A，本文作为 v1 配置/密钥/迁移基线。
> 输入：04（Tabby config/vault/importer 研究）、07（v1 验收的迁移节）、12（设置页/UI 架构）、10（配置根/平台）。
> 一句话：v1 用 TOML 完整规范化配置 + 独立 state.json；密钥默认 OS keychain、可选加密 vault；Tabby 迁移走 detect → plan → apply，只读原文件、逐项确认、默认不落明文。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A config.toml + state.json；Q2=A 显式规范化 + schema_version + 类型化 profile + options map；Q3=A keychain 默认 + 可选 vault；Q4=A Argon2id + XChaCha20-Poly1305 每 secret AEAD；Q5=A importer detect→plan→apply，不迁 plugins/tabsRecovery/analytics；Q6=A 明文密码默认跳过、逐条确认，vault 一次性解密；Q7=A 配置根优先级 + 无头用 vault + 解锁按需/15 分钟自动锁。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | 配置格式/布局 | A. config.toml（TOML，完整规范化，人可读）+ state.json（窗口/恢复/最近，机器状态）+ 可选 vault.enc；不沿用 Tabby 的 YAML 稀疏覆盖 | B. YAML（贴近 Tabby，但稀疏/锚点/Norway 问题）；C. JSON（机器友好，人不友好） |
| Q2 | 配置模型 | A. 显式规范化 + schema_version + 类型化 profile（local/ssh/serial/telnet）+ group/defaults + 每 provider 的 options map；未知 provider 类型跳过并进报告；不静默丢字段 | B. 照搬 Tabby 的 type + options 松散结构 + 稀疏覆盖：迁移容易，调试难；C. 最小配置：多 profile/迁移不好落 |
| Q3 | 密钥存储 | A. 默认 OS keychain（macOS Keychain / Windows Credential Manager / Linux Secret Service）；可选加密 vault 作为便携/无头/跨设备容器 | B. 只用 vault：没启用 vault 的用户每次都要口令；C. 只用 keychain：无头/便携/跨平台同步难 |
| Q4 | vault 加密 | A. Argon2id + XChaCha20-Poly1305，envelope + 每 secret 独立 AEAD（AAD 绑 name/type），KDF 参数版本化；Legacy TabbyVault 仅一次性解密 | B. age：标准但多一层格式；C. 继续 Tabby 的 PBKDF2-SHA512 100k + AES-256-CBC：无 MAC，不推荐 |
| Q5 | importer 范围 | A. detect → plan → apply；profiles/groups/defaults/known_hosts/配色/快捷键/可映射 terminal 设置自动 + 审阅；vault 一次性解密；不迁 plugins/tabsRecovery/analytics/electronFlags/hacks；recentProfiles 可选；B. 连 recentProfiles/LevelDB 也解析；C. 只迁 profiles |
| Q6 | 秘密迁移 | A. 明文 password / scripts[].send 默认不导入，逐条确认；vault 需口令后一次性解密并重加密；keytar 条目不静默读，需用户授权或手输；报告列出需重输项 | B. 自动导入所有明文密码：泄露风险；C. 完全不迁秘密：用户要全部重输 |
| Q7 | 路径/无头/解锁 | A. 配置根优先级 --config-dir > RABBY_CONFIG_DIRECTORY > exe 同目录 data/（便携）> 平台标准目录；keychain 默认、vault 用于无头/便携；解锁按需 + 空闲 15 分钟自动锁定；B. 只用平台标准目录；C. 只用 vault |

## 1. 配置布局与模型

    <config-dir>/
      config.toml        # 用户可编辑：profiles/groups/defaults/hotkeys/terminal/appearance/ssh/privacy
      state.json         # 机器状态：窗口 bounds、tabs 拓扑、recent、first_run、update_check
      vault.enc          # 可选：加密 secret 容器
      known_hosts.json   # 可选：Rabby 自己的 host key 存储（也可放 config）
      logs/              # 脱敏日志，按大小滚动

- 配置根：--config-dir > RABBY_CONFIG_DIRECTORY > exe 同目录 data/（便携）> 平台标准目录（macOS Application Support、Windows AppData、Linux XDG）。
- 写盘：临时文件 + 原子 rename；保留 N-1 备份；文件权限 0600（Windows 用 ACL）; 不用 Tabby 的双写 config + .backup 语义。
- schema_version 整数；启动时按版本迁移；迁移前备份；不认识的未来版本拒绝启动并提示。
- profile：id、type（local/ssh/serial/telnet）、name、group、options（按 type 类型化；provider 特有字段放 options map，保留未知字段但不执行）。
- groups：树 + parent_id；group defaults 按 provider 合并；profileDefaults 全局默认；合并顺序：内置默认 → profileDefaults → group → profile。
- hotkeys：action id → chord 数组；平台默认表；冲突检测；导入时映射 Tabby 动作名，未命中列报告。
- terminal/appearance/ssh/privacy：显式写出生效值；隐私默认零遥测；state.json 可删，不影响配置。
- 未知/插件 profile 类型：跳过并保留原始 JSON（脱敏）到导入报告，供手动重建；不写进 config。
- 配置文件不存任何密钥；密码/口令/私钥 passphrase 只进 secret store。

## 2. Secret store

    trait SecretStore {
        fn get(&self, key: SecretKey) -> Result<SecretString>;
        fn set(&mut self, key: SecretKey, value: SecretString) -> Result<()>;
        fn delete(&mut self, key: SecretKey) -> Result<()>;
        fn list(&self) -> Result<Vec<SecretMeta>>;
    }

- keychain backend：keyring crate；service 名 rabby；account 用 profile id + secret type；Linux Secret Service 不可用时提示切 vault；CI 用 mock。
- vault backend：
  - 文件头：magic、format_version、KDF 参数（Argon2id m/t/p、salt）、nonce 前缀；每 secret 条目含 name/type/created/updated + 密文。
  - 每 secret 独立 AEAD（XChaCha20-Poly1305），AAD = name + type + format_version；解锁时一次派生 key，逐条解密。
  - 解锁按需；内存里用 secrecy/zeroize；空闲 15 分钟或窗口锁定时丢弃 key；不写临时明文。
  - KDF 参数版本化；升级参数时保留旧版本解密路径，后台重加密。
- Legacy TabbyVault：PBKDF2-HMAC-SHA512 100k + AES-256-CBC（无 MAC）；只读一次性解密；导入后重加密；提示旧 vault 仍在磁盘（含 .backup），建议用户自行删除或保持。
- keytar 条目：不静默跨 app 读取；由用户在钥匙串授权或手输；导入报告列出。
- secret 类型 v1：SSH password、private key passphrase、login script secret、proxy password、vault passphrase 不存（内存/用户输入）。
- 安全：不 Debug 打印 secret；日志/错误脱敏；导入器用 secrecy 包装内存明文；不落临时文件；崩溃转储不含 secret；文件权限 0600。

## 3. Tabby importer（rabby-import-tabby）

三阶段：
1. detect：平台配置目录、TABBY_CONFIG_DIRECTORY、便携 data/；识别 config.yaml、config.yaml.backup、vault、userData/plugins、LevelDB。
2. plan：只读解析 → ImportPlan（强类型、可 JSON）；逐项标记 auto/confirm/skip；未知字段保留原始 JSON（脱敏）；冲突检测（同名 profile、快捷键冲突、已有 Rabby 配置）。
3. apply：用户逐组确认 → 备份 Rabby 配置 → 写 config/secret store → 报告；失败回滚；绝不写 Tabby 文件。

映射（来自 04）：
- profiles（local/ssh）：自动 + 审阅；options.privateKeys 自动；password/scripts[].send 默认跳过并确认；未知 type 跳过 + 报告。
- groups/profileDefaults：自动；处理 __nonStructural 的整组替换语义。
- hotkeys：动作映射表 + 冲突检测；未命中报告。
- appearance.theme：按名字匹配预设，未命中回退；appearance.css 无等价物，报告。
- terminal.*：可映射项自动；frontend/font/paletteGenerate 等内核相关项跳过并报告。
- colorScheme/lightColorScheme/customColorSchemes：自动导入 TerminalPalette。
- ssh.*：agentType/agentPath/x11Display 等按平台映射；knownHosts 自动 + 指纹提示，并建议顺带导入 OpenSSH known_hosts。
- vault secrets：口令后一次性解密 → 重加密；逐条审阅；旧 vault 不删除。
- keytar：不可自动；用户授权/手输。
- recentProfiles：可选（LevelDB）；tabsRecovery 不迁；plugins/configSync/analytics/electronFlags/hacks 丢弃并报告。
- 幂等：config 记 imported_from = tabby@版本；再次导入默认只补新增，不覆盖用户改动；报告列出变化。

## 4. UI（与 12 对齐）

- 设置页：常规、终端外观、profiles、快捷键、配色、隐私/安全；vault 状态/锁定/修改口令。
- 导入向导：detect 结果 → plan 分组确认（profiles/colors/hotkeys/secrets）→ 应用 → 报告；支持 dry-run。
- vault 解锁对话框：按需弹出，显示来源（哪个 secret）；错误口令/损坏文件有明确提示。
- 无头/CLI：rabby import-tabby --dry-run/--apply；vault passphrase 从 stdin/环境变量（不写 shell history）。

## 5. 测试与验收

- 配置：roundtrip、schema 迁移、未知字段保留、原子写/备份、权限。
- 加密：Argon2id/XChaCha20-Poly1305 已知答案测试；vault 损坏/错误口令/重放/AAD 篡改；Legacy TabbyVault 测试向量（PBKDF2/AES-CBC）。
- keychain：mock backend；macOS/Windows/Linux 手工矩阵；Secret Service 不可用降级。
- importer：04 的真实 Tabby fixture（含 vault/SSH/配色/快捷键/未知插件 profile）→ 0 P0 数据丢失；跳过项报告 100%；重复导入幂等；回滚。
- 安全：日志/错误无 secret；临时文件无明文；进程内存检查（尽力）；旧 vault 提示。
- 验收对齐 07 §6：真实 Tabby 配置导入 0 P0 数据丢失；所有跳过项可见；用户可审阅/取消。

## 6. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| keychain 跨 app ACL/headless 不可用 | 迁移/无头体验 | vault 兜底；用户授权/手输；CI mock |
| Tabby vault 弱 KDF/无 MAC | 旧数据风险 | 只读一次性解密、立即重加密、提示口令强度/删除旧 vault |
| 配置 schema 演进 | 旧版本读新配置 | schema_version + 迁移 + 拒绝未来版本 + 备份 |
| 导入数据丢失 | 用户不信任 | dry-run + 逐项确认 + 报告 + 回滚 + 不写源文件 |
| TOML 对任意 provider options 的表达 | 复杂嵌套 | options 用 map/值；类型化核心字段；未知字段保留报告 |
| vault 文件丢失/忘记口令 | 秘密不可恢复 | 文档说明 + 可选 keychain 备份；不承诺恢复 |

## 7. ADR 草稿要点

- ADR-0005：config.toml 完整规范化 + state.json + schema_version；不沿用 Tabby 稀疏 YAML。
- ADR-0006：secret 默认 OS keychain，vault 为可选容器；Argon2id + XChaCha20-Poly1305，每 secret AEAD。
- ADR-0007：Tabby importer detect → plan → apply，只读、逐项确认、不迁插件/会话；Legacy TabbyVault 只读解密。

## 8. 证据

- 04：.scratch/rust-rewrite/research/04-tabby-config-vault-compat.md（schema/vault/importer/安全）
- 07：.scratch/rust-rewrite/specs/v1-scope.md §6
- 12：.scratch/rust-rewrite/specs/ui-architecture.md（设置页/导入向导）
- 10：.scratch/rust-rewrite/specs/platform-distribution.md（配置根/便携）
