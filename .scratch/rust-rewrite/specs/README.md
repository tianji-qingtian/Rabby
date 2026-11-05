# Rabby 蓝图索引（地图 00 已完成）

> 2026-11-05：地图 00 的 17/17 张子票全部闭环，Decisions so far 17 行。地图：../map.md；研究笔记：../research/；原型：../prototype/。
> 本目录是交给实施团队的规格集合；本 map 不写产品代码。

## 决策文档

| 文档 | 管什么 |
|---|---|
| [v1-scope.md](v1-scope.md) | v1 产品边界、P0/P1/P2 范围、18 条隐性行为、14 项 xterm.js 契约、性能门槛、迁移验收 |
| [platform-distribution.md](platform-distribution.md) | 平台首发顺序、安装包、签名/公证、品牌、许可、发布节奏、Web/移动 |
| [09-prototype-report.md](09-prototype-report.md) | GPUI Kit + wezterm-term + portable-pty 实测：吞吐/内存/PTY resize/CJK/帧时间 |
| [terminal-core-rendering.md](terminal-core-rendering.md) | wezterm-term + TerminalCore façade、UI 线程解析/字节预算、TerminalElement、脏行 diff、键鼠/IME、兼容测试 |
| [ui-architecture.md](ui-architecture.md) | Workspace/PaneTree/TabStrip、TerminalPane 边界、Kit 组件映射、主题、无障碍/IME、NSH 边界 |
| [config-secrets-migration.md](config-secrets-migration.md) | config.toml/state.json、profile/group 模型、keychain/vault、Legacy TabbyVault、importer |
| [extension-security.md](extension-security.md) | gpui-shell/Wasmtime 分工、rabby-ext-api、权限矩阵、签名/分发、Tabby 插件映射 |
| [platform-integration.md](platform-integration.md) | rabby-platform、shell 目录、ConPTY/UAC/Clink、v1/v1.x 系统能力、热键/Quake/托盘、通知/URL/拖放 |
| [engineering-architecture.md](engineering-architecture.md) | 13 crate workspace、runtime/线程、工具链 pin、CI、错误/日志、feature/版本 |
| [delivery-plan.md](delivery-plan.md) | 5–6 人 9–12 个月、M0-M4、团队/并行、beta/v1.0 门槛、风险、上游对齐 |

## 一句话结论

v1 = 可日用的 Rust 桌面终端（本地 shell + SSH + 标签/分屏 + 设置/主题 + 拓扑级恢复 + Tabby 迁移）；GPUI Kit 外壳 + 自研 TerminalElement；wezterm-term + portable-pty；config.toml + keychain/vault；扩展 v1.x；Quake/托盘/热键 v1.x；5–6 人 9–12 个月。

## 实施入口（M0）

1. 建 workspace + 13 crate 骨架 + rust-toolchain nightly-2026-11-04 + CI。
2. TerminalCore façade + TerminalElement 最小版（GPUI 窗口 + 本地 PTY + 文字）。
3. AppShell/TerminalPane/PaneTree 最小版。
4. 配置/密钥骨架 + rabby-platform 骨架。
5. 第一个内部 dogfood：macOS 开窗、跑 zsh、输入/resize。

## 验收锚点

- 07 的 P0/P1 与 18 条隐性行为、14 项 xterm.js 契约。
- 09 的性能基线 + 11 的 10MB/s@60fps、p95 ≤16ms、8 标签 ≤300MB。
- 13 的迁移 0 P0 数据丢失 + 跳过项报告。
- 08 的 macOS 签名/公证、三平台安装包。
- 17 的 M1/M3/M4 门槛。

## 地图

- 地图：../map.md（Destination / Notes / Decisions so far / Not yet specified / Out of scope）。
- 17 张子票：../issues/（全 resolved）。
- 研究：../research/（01–10 的 7 份笔记）。
- 原型：../prototype/（09-hello / 09-highload / 09-anim 截图）。
