<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:grilling -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T16 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.237Z -->
<!-- DSH-BLOCKED-BY: 11, 12, 13, 14, 15 -->
<!-- DSH-ASSIGNEES: lead -->
# 16: 工程架构与 crate 划分

Type: grilling

Blocked by: 11, 12, 13, 14, 15
Status: resolved
## Question

目标：把前述决策收敛成可开工的 Rust 工程架构。

要讨论：
- cargo workspace 与 crate 边界（core/terminal/pty/ssh/serial/config/vault/ui/platform/plugin/app）；
- 异步运行时（tokio？）、线程模型、GPUI entity 与服务生命周期；
- 事件/命令/配置的跨层契约；headless 测试边界；
- 错误处理/日志/遥测接口；feature flags；
- 依赖策略、MSRV、许可证审计、CI 矩阵；
- 性能预算与基准落点；
- 与 gpui-kit/nsh 的版本锁定策略。

产出：crate 图 + 关键 trait/接口草图 + ADR 清单，写回 Answer。

## Comments

### local — 2026-11-05T03:32:42.237Z
草案 v0.1 已落盘：.scratch/rust-rewrite/specs/engineering-architecture.md。含 workspace/crate 图、关键 trait、运行时/线程模型、工具链/依赖/许可、测试/CI、错误/日志、feature/版本；待拍板 §0 的 7 项。

## Answer

工程架构：约 13 个 crate 的 workspace（core/config/secrets/import-tabby/terminal/pty/ssh/session/platform/terminal-view/ui/ext-api/ext-host/app/test-support）；rabby-terminal-view + rabby-ui 拆分；GPUI 主线程 + 专用 tokio + 每会话 PTY reader，v1 解析在 UI 线程；nightly-2026-11-04 + 精确 pin + vendor pathfinder_simd + cargo-deny/audit；三平台 CI + unit/golden/PTY/SSH/UI/bench；thiserror/anyhow + tracing + 脱敏 + 零遥测；v1.x 可选 crate/feature + workspace SemVer + ext-api 独立版本。详见 .scratch/rust-rewrite/specs/engineering-architecture.md

## 进度

- 进度：40%（2026-11-05T11:32:42+08:00）：工程架构草案已写；等用户拍板 §0 的 7 项。
