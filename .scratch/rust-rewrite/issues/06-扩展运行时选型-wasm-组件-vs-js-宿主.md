<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T06 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.236Z -->
<!-- DSH-ASSIGNEES: lead -->
# 06: 扩展运行时选型：WASM 组件 vs JS 宿主

Type: research

Status: resolved
## Question

目标：为 Rabby 的第三方扩展机制选运行时，替代 Tabby 的 Angular 插件。

要回答：
- 候选：Wasmtime/Component Model、Extism、wasm3、QuickJS/rquickjs、gpui-shell（JS 扩展，权限逐项授予）、原生 dylib；
- 能力：UI 扩展（挂面板/菜单/命令）、后台逻辑、网络/文件/进程权限、沙箱、签名与分发、热重载、性能、跨平台、调试体验；
- 与 GPUI/GPUI Kit 的集成点（UI 由宿主绘制还是扩展自绘）；
- 迁移路径：Tabby 常见插件（docker、title-control、quick-cmds、save-output、sync-config、sftp-tab、highlight、mcp-server）分别对应什么新形态；
- 安全模型：能力声明、签名、更新、撤销；
- 推荐 1 个主运行时 + 1 个兜底。

产出：`.scratch/rust-rewrite/research/06-extension-runtime.md`。

## Answer

主运行时选 gpui-shell（QuickJS + 声明式 UI 描述树 + capability manifest），兜底 Wasmtime + Component Model 只承载无 UI 的重活。要点：① Tabby 插件实为全权限 Node 程序（npm 193 个包、零校验、Arborist 直装），新机制必须给出等价 UI 表达力；② WASM 每次调用过 canonical ABI，做逐帧 UI 是负优化，Zed/Extism 的先例都只做 headless；③ gpui-shell 与 GPUI Kit 同源、可原位嵌入（ShellRuntime::load → ShellRoot），443 节点描述 1.4ms 且未变更帧 0 次进 VM，热重载/沙箱/调用帧级权限均已具备；④ 但它自认缺两块，须由 Rabby 自建：授权产品（granted/denied/prompt + 更新加权限再询问）与贡献注册表（commands/keymaps/panels）；⑤ 版本 0.9.0、自标 preview，扩展 ABI 必须定义在 Rabby 自有 crate 里。研究笔记：.scratch/rust-rewrite/research/06-extension-runtime.md

## 进度
