<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T05 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.236Z -->
<!-- DSH-ASSIGNEES: lead -->
# 04: Tabby 配置、密钥与会话兼容面

Type: research

Status: resolved
## Question

目标：搞清楚从 Tabby 迁移到 Rabby 需要读哪些数据、哪些能自动导入、哪些必须重新加密或用户确认。

要回答：
- config.yaml 的 schema、profile/group/provider 模型、平台默认值、命令/快捷键、配色方案格式、插件配置；
- vault：加密格式（算法、KDF、字段）、master passphrase、secret 类型、keytar/系统钥匙串的使用；
- known_hosts、SSH 私钥位置/agent、WinSCP 导入、会话恢复（tab recovery）数据结构；
- Tabby CLI / 环境变量 / 便携模式依赖的外部文件；
- importer 的输入/输出映射与不可自动迁移项清单；
- 安全注意：迁移过程中明文密钥的落盘/内存风险。

产出：`.scratch/rust-rewrite/research/04-tabby-config-vault-compat.md`。

## Answer

迁移面只有三处真源：config.yaml（+ .backup）、userData/plugins、Electron localStorage(LevelDB)，没有独立密钥库；导入器应做字段级翻译并以 Rabby 默认值兜底（Tabby config 是稀疏覆盖层，缺键=默认值）。
vault = PBKDF2-HMAC-SHA512×100k、8B salt + AES-256-CBC、无 MAC（vault.service.ts:13-18,69-98），只能"一次性解密导入"，Rabby 应改用 Argon2id + AEAD；私钥口令的 vault key 是私钥文件内容的 sha512，换方案即需用户重输。
known_hosts 存在 config 的 ssh.knownHosts（digest=base64(sha256(key blob))），Tabby 从不读 ~/.ssh/known_hosts，可双向映射；票面「WinSCP 导入」不成立——本基准与 v1.0.190 均只有 WinSCP 启动集成。
会话恢复不可迁移：localStorage.tabsRecovery 的 savedState 是 xterm 私有序列化（scrollback 1000），v1 只迁 recentProfiles。
明文风险 4 处：options.password、scripts[].send、config.yaml.backup、启用 vault 后未清理的 keytar 条目；导入器需默认不落明文。
研究笔记：.scratch/rust-rewrite/research/04-tabby-config-vault-compat.md

## 进度
