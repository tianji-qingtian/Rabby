# Rabby 交付分期与里程碑（草案 v0.1）

> 状态：17 票「交付分期与里程碑」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A；这是地图的最后一票，蓝图已完成。
> 输入：07（v1 范围/验收）、08（平台/分发）、09（原型数据）、11–16（内核/UI/配置/扩展/平台/架构）。
> 一句话：5–6 人、约 9–12 个月到 v1.0；M0 骨架 → M1 本地终端可用（macOS alpha）→ M2 SSH+迁移+Windows alpha → M3 beta（三平台 P0）→ M4 v1.0；v1.x 按 SFTP → 串口/Telnet/Zmodem → 扩展 → Quake/托盘/热键/便携 → 自动更新。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A 5–6 人、9–12 个月；Q2=A M0-M4 里程碑；Q3=A M1 macOS alpha / M3 三平台 beta / M4 v1.0；Q4=A v1.x SFTP→串口/Telnet/Zmodem→扩展→Quake/托盘/热键/便携→自动更新；Q5=A 风险前三 GPUI 供应链/终端渲染 IME/Windows shell+ConPTY；Q6=A ADR/spec/CI/dogfood/周会/安全审查；Q7=A 锁 Tabby 467ce8e + 季度 parity audit。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | 团队与周期 | A. 5–6 名工程师，约 9–12 个月到 v1.0（±30%） | B. 3–4 名，12–15 个月；C. 8+ 名，6–9 个月（协调成本/招聘风险） |
| Q2 | 里程碑形状 | A. M0 骨架 → M1 本地终端（macOS alpha）→ M2 SSH+迁移+Windows alpha → M3 beta → M4 v1.0；每期可运行、可验收 | B. 按功能纵向切（先 SSH 再 UI）：难持续 dogfood；C. 大爆炸：风险最高 |
| Q3 | Beta 门槛 | A. M1 macOS 内部 alpha；M3 三平台 beta（P0 全过、迁移 0 P0 丢失、8h soak、macOS 已签名、安全审查）；M4 v1.0 三平台 P0 无缺项 | B. v1.0 只保证 macOS；C. 每个里程碑都要求三平台全功能：拖慢 |
| Q4 | v1.x 顺序 | A. SFTP → 串口/Telnet/Zmodem → 扩展 → Quake/托盘/全局热键/完整便携 → 自动更新/市场 | B. 扩展优先：生态早，但 v1 核心未稳；C. 全部延后：失去差异化 |
| Q5 | 风险优先级 | A. 前三：GPUI/gpui-pre 供应链与工具链、终端渲染/IME/CJK、Windows shell/ConPTY | B. SSH/迁移优先；C. 发布/签名优先（可并行处理） |
| Q6 | 工程流程 | A. ADR + spec + CI + 内部 dogfood + 每周 demo + release checklist + 安全审查 | B. 轻量流程；C. 无流程 |
| Q7 | 上游对齐 | A. 锁定 Tabby 467ce8e；每季度做一次 parity audit；不复制代码、保留 MIT 归属；v1 范围冻结 | B. 持续追 latest：范围漂移；C. 不对齐：功能差距失控 |

## 1. 里程碑

| 里程碑 | 时间（5–6 人） | 目标 | 出口条件 |
|---|---|---|---|
| M0 骨架 | 0–6 周 | workspace/13 crate、CI、nightly pin、TerminalCore façade、TerminalElement 最小版、本地 PTY、AppShell/TerminalPane | macOS 内部可开窗、跑 zsh、输入/resize；CI 绿；ADR 0001–0010 |
| M1 本地终端可用（macOS alpha） | 6–18 周 | tabs/splits/PaneTree、设置/主题、会话恢复、shell 目录、剪贴板/粘贴、title/cwd/progress、搜索 P1 | 内部团队日用 1 周；8h soak；终端 P0 子集；macOS alpha |
| M2 SSH + 迁移 + Windows alpha | 18–30 周 | SSH 认证/jump/转发/known_hosts/agent；config/secrets/importer；Windows shell/ConPTY；Linux alpha | SSH 验收矩阵；迁移 0 P0 数据丢失；Windows/Linux 内部 alpha；跨平台 CI |
| M3 beta | 30–40 周 | 三平台 P0、性能 10MB/s@60fps、IME/CJK 矩阵、无障碍冒烟、macOS 签名/公证、安装包、文档 | 公开 beta 门槛（§3）；已知问题清单 |
| M4 v1.0 | 40–44 周 | bug bar、soak、迁移 fixture、发布/回滚 | 三平台 P0 无缺项；1 周 dogfood；v1.0 |
| v1.x | M4 之后 | SFTP → 串口/Telnet/Zmodem → 扩展 → Quake/托盘/热键/便携 → 自动更新/市场 | 每项独立 proposal/验收 |

- 时间含缓冲；5–6 人、scope 冻结在 07 的 v1。3–4 人按 12–15 个月；8+ 人按 6–9 个月（协调成本上升）。
- 关键路径：M0 → TerminalCore/Element + UI 壳 → M1 alpha → SSH/迁移 + Windows → M2 → beta → v1.0。

## 2. 团队角色与并行工作流

| 角色 | 主要职责 | 从何时 |
|---|---|---|
| Tech Lead/架构 | ADR、跨 crate 接口、review、风险 | M0 |
| 终端内核/渲染 | TerminalCore、wezterm 适配、TerminalElement、脏行/性能/IME | M0 |
| UI/外壳 | Workspace/PaneTree/TabStrip/AppShell/设置/命令/主题 | M0 |
| 网络/SSH | russh、认证/jump/转发/known_hosts/agent、SFTP(v1.x) | M1 |
| 平台/配置 | rabby-platform、shell/ConPTY、config/secrets/importer、打包 | M0 |
| QA/发布 | CI、测试矩阵、性能/soak、签名/安装包、release | M1 |
| 产品/设计 0.5 | 交互/验收/品牌/文档 | M0 |

并行：
- M0：TL + 终端 + UI + 平台；QA 搭 CI。
- M1：终端/渲染 + UI + 平台（macOS/Linux）；QA 建矩阵。
- M2：网络/SSH + 配置/迁移 + 平台 Windows；UI 打磨；QA 扩三平台。
- M3：QA 主导 beta 门槛；终端做性能/IME；平台做签名/安装包；所有人修 P0。
- M4：QA/发布主导；TL 守 bug bar。

依赖：
- M0 是所有前置；TerminalCore/Element 阻塞终端 pane；config/secrets 阻塞 SSH 凭据/迁移；platform shell/PTY 阻塞本地终端；迁移依赖 config/secrets；beta 依赖全部。

## 3. Beta / v1.0 门槛

Alpha（M1）：
- macOS 内部；本地 shell；tabs/splits；8h soak；无迁移；P0 终端子集。

Beta（M3，对外）：
- macOS + Windows + Linux（Linux alpha 可标 known issues）。
- 07 的 P0 全过；迁移真实 Tabby 配置 0 P0 数据丢失；跳过项 100% 报告。
- 性能：10MB/s@60fps、输入 p95 ≤16ms、8 标签 ≤300MB；09 基线校准。
- IME/CJK 三平台手工矩阵通过；无障碍键盘/AccessKit 冒烟。
- macOS 已签名/公证；Windows 未签名有 SmartScreen 说明；Linux AppImage/deb。
- 安全审查：secrets/vault/importer、URL/拖放、插件无 v1；无 P0 安全问题。
- 文档：安装/迁移/已知问题/快捷键/FAQ。

v1.0（M4）：
- 三平台 P0 无缺项；1 周内部 dogfood；8h soak 无崩溃/泄漏；迁移 fixture 全过；回滚方案；发布 checklist。

## 4. 风险与缓解

| 排名 | 风险 | 影响 | 缓解 |
|---|---|---|---|
| 1 | GPUI/gpui-pre 供应链/工具链（nightly、pathfinder_simd、Kit pin） | 构建断/升级返工 | rust-toolchain 固定 + 精确 pin + vendor 补丁 + CI + fork 预案；09 已验证 |
| 2 | 终端渲染/IME/CJK/性能 | 日用体验/性能不达标 | 09 原型基线；自定义 Element + 脏行；256KB/tick；IME 三平台矩阵；worker 作为 fallback |
| 3 | Windows shell/ConPTY | Windows 用户无法日用 | 早接 Windows CI；shell 矩阵；系统 ConPTY + Job Object；VS vswhere；UAC v1.x |
| 4 | 迁移/密钥 | 用户数据丢失/泄露 | dry-run/逐项确认/回滚；fixtures；secrecy/zeroize；安全审查 |
| 5 | 签名/品牌/发布 | 发布延期 | Apple 账号提前 4–6 周；公开名前商标检索；Windows 证书 v1.x |
| 6 | scope creep | 延期 | 07 的 P0/P1 冻结；v1.x 单独立项；扩展 v1.x |

## 5. 上游 Tabby 对齐

- 锁定上游 467ce8e；每季度 parity audit（README/源码/配置/行为），更新兼容矩阵。
- 不复制 Tabby 代码；行为对齐 + MIT 归属；插件不兼容（14）。
- v1 范围冻结在 07；v1.x 再评估上游新增能力。

## 6. 工程流程

- ADR + spec 先行；每个 crate 有 owner/reviewer。
- CI 门：fmt/clippy/test/deny/audit/build；三平台。
- 内部 dogfood 从 M1 开始；每周 demo；release checklist；安全审查在 beta 前。
- 版本：nightly 构建 + stable tag；v1 手动更新；v1.x 自动更新（08）。

## 7. 估算与不确定性

- 5–6 人：9–12 个月到 v1.0；±30%。
- 3–4 人：12–15 个月；8+ 人：6–9 个月（协调/招聘风险）。
- 最大不确定性：GPUI 上游、IME/CJK、Windows shell/ConPTY、迁移密钥；用 M0/M1 的实测尽早降级。

## 8. 地图收口

- 17 关票后，地图 00 的 17 张子票全部关闭；Decisions so far 17 行；specs/ 下有 10 份决策文档。
- 下一步：由实施 effort 从 M0 开始；本 map 不写产品代码（除非目的地改为执行型）。
- 用户可在地图详情页做最终核对并标记完成。

## 9. 证据

- 07：.scratch/rust-rewrite/specs/v1-scope.md
- 08：.scratch/rust-rewrite/specs/platform-distribution.md
- 09：.scratch/rust-rewrite/specs/09-prototype-report.md
- 11–16：terminal-core-rendering / ui-architecture / config-secrets-migration / extension-security / platform-integration / engineering-architecture
