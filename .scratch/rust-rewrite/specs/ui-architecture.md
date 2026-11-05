# Rabby UI 架构与 NSH 边界（草案 v0.1）

> 状态：12 票「UI 架构与 NSH 边界」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A，本文作为 v1 UI 基线。
> 输入：02（GPUI/GPUI Kit/NSH 能力）、09（垂直切片实测）、07（v1 验收基线）、08（平台/分发）；GPUI Kit 0.9 的 Dock/Settings/Command/TabBar 源码与 examples/dock。
> 一句话：v1 用 GPUI Kit 做外壳与通用 UI，终端拓扑与终端 widget 自研；NSH 不进 v1 主路径。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A 自研 Workspace/PaneTree/TabStrip；Q2=A 顶部标签 + 四向嵌套分屏 + 最大化（四边/Quake/多窗口 v1.x）；Q3=A TerminalPane 拥有 TerminalCore/PTY/焦点/选区/滚动；Q4=A Kit 做 Settings/Command/通知/菜单，终端与 profile/迁移逻辑自研；Q5=A Kit token 外壳 + 独立 TerminalPalette；Q6=A v1 无障碍/IME 冒烟 + 三平台手工矩阵；Q7=A NSH 不进 v1，v1.x 只评估扩展/运行时 UI。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | 终端拓扑 | A. 自研 Workspace + PaneTree + TabStrip（精确复刻 Tabby 的窗口标签/嵌套分屏/拖拽落区/会话恢复）；Kit 只提供 TabBar 外观、按钮、菜单、分屏把手等零件 | B. 直接用 Kit DockArea/DockLayout 做终端布局：省外壳代码，但窗口标签=面板、标签内含分屏树的模型要硬套，Tabby 落区/恢复语义要另写；C. 混合（自定义窗口标签 + 每个标签内 Kit DockArea）：中间路线，状态与恢复更复杂 |
| Q2 | v1 标签/分屏范围 | A. 窗口标签在顶部；标签内四向嵌套分屏；拖动重排/重命名/关闭/最大化单 pane/焦点切换；四边标签栏、Quake、多窗口、广播输入放 v1.x | B. 四边标签栏 + Quake + 多窗口也进 v1：平台与交互风险显著上升；C. 只做单层分屏：不满足 Tabby 日用 |
| Q3 | 终端 widget 边界 | A. TerminalPane 实体拥有 TerminalCore（wezterm）+ PTY + 焦点 + 选区 + 滚动；TerminalElement 自绘；PaneTree 只管拓扑；Workspace 只管标签；AppShell 只管窗口/设置/命令 | B. 由 AppShell 直接持有终端状态：壳与终端耦合，分屏/恢复难做；C. 终端独立进程：跨进程输入/渲染成本高，v1 不做 |
| Q4 | 通用 UI 组件映射 | A. Settings/SettingPage/Group/Field、Command 面板、通知/toast、菜单/对话框/输入用 Kit；终端、profile 编辑器、迁移向导的逻辑自研（表单字段复用 Kit） | B. 连 profile 编辑器/迁移向导也全用 Kit 拼：表单能力够，但业务状态与校验仍要自研；C. 全部自研：重复造轮子 |
| Q5 | 主题与配色 | A. 外壳用 Kit 语义 token / Theme；终端用独立 TerminalPalette（16 色 + fg/bg/cursor）；Tabby color scheme 导入到 TerminalPalette；不做 CSS 主题/vibrancy | B. 自建 token 系统：控制力强但重复 Kit；C. 支持 CSS 主题：Rust/GPUI 无等价 DOM，v1 不做 |
| Q6 | 可访问性与 IME | A. v1 做键盘可达 + Kit AccessKit 外壳 + 终端 AccessKit 文本/选区冒烟；三平台 CJK/IME 手工矩阵；完整无障碍审计放 v1.x | B. 全部放 v1.x：与 07 的键盘可达冲突；C. v1 做完整审计：终端文本语义工作量大，拖慢 v1 |
| Q7 | NSH 角色 | A. v1 不引入 NSH；v1.x 只评估它做扩展/运行时可下发 UI（需 Kit 切 gpui-fast 的 PoC），不做核心外壳/终端 | B. NSH 做 Web/移动端外壳；C. NSH 进 v1 主路径：02 结论是 0.4.0 太新、gpui-fast/gpui-pre 双栈风险，不建议 |

## 1. 分层与所有权

    AppShell（窗口、菜单、快捷键、设置、命令面板、通知、迁移向导）
      └─ Workspace（窗口标签、活动标签、会话恢复序列化）
           ├─ TabStrip（Kit TabBar 外观 + 自定义溢出/拖动/重命名）
           ├─ Tab
           │    └─ PaneTree（SplitNode 方向/比例/子节点；Leaf = TerminalPane）
           │         └─ TerminalPane（TerminalCore + PTY + 焦点 + 选区 + 滚动）
           │              └─ TerminalElement（GPUI 自定义 Element：网格、光标、选区、滚动）
           ├─ SettingsView（Kit Settings）
           ├─ CommandPalette（Kit Command）
           └─ Notifications/Toasts（Kit + rabby-platform 系统通知）

- TerminalCore：headless VT + 屏幕模型 + 键鼠编码（11 的产出）；不依赖 GPUI。
- TerminalPane：唯一持有 PTY、焦点 handle、选区、滚动位置、搜索状态的实体；把 GPUI 键鼠事件翻译成 core 输入；把 core 的脏行/快照交给 TerminalElement。
- PaneTree：只处理 split 拓扑、比例、焦点在 pane 间的移动、最大化；不碰终端内容。
- Workspace：窗口标签、活动标签、恢复；不碰渲染细节。
- AppShell：窗口生命周期、菜单、快捷键分发、设置页、命令面板、通知、迁移向导；不直接持有终端。
- 这样 11 可以独立做 TerminalCore + TerminalElement，12/16 做 PaneTree/Workspace/AppShell。

## 2. 组件映射

| 功能 | v1 方案 | Kit 组件/自研 |
|---|---|---|
| 窗口/标题栏 | GPUI 原生窗口 + Kit 按钮/菜单；macOS 交通灯保留 | Kit 按钮/菜单；标题栏布局自研 |
| 窗口标签 | 顶部标签、拖动重排、重命名、关闭、溢出滚动 | TabBar 外观；Workspace 数据模型自研 |
| 分屏 | 四向嵌套 SplitNode、拖动把手、最大化单 pane | 自定义 PaneTree；把手用 Kit resizable/自绘 |
| 终端 | 网格、光标、选区、滚动、URL、搜索高亮 | 自研 TerminalElement（11） |
| 设置 | 常规、终端外观、profiles、快捷键、配色 | Kit Settings/SettingPage/Group/Field |
| 命令面板 | 命令注册、模糊搜索、快捷键提示 | Kit Command/CommandState |
| 通知 | 应用内 toast + 系统通知（默认关） | Kit toast；系统通知走 rabby-platform |
| Profiles | 本地/SSH 列表、分组、编辑器、默认值 | 逻辑自研；List/Table/Input/Dialog 用 Kit |
| 配色方案 | 终端 16 色 + fg/bg/cursor 编辑器、Tabby 导入 | 自研 palette 模型；字段/颜色选择用 Kit |
| 迁移向导 | detect → plan → apply、逐组确认、报告 | 逻辑自研；Dialog/Form/Step 用 Kit |
| 选择器/模糊查找 | profile 选择、最近、命令 | Kit Command/List/Input |
| Quake/托盘/全局热键 | v1.x | 平台扩展（10/15），不用 Dock |
| Dock 面板 | v1 不用 DockArea 做终端拓扑；SFTP 等侧栏 v1.x 再评估 | Kit Dock 作为 v1.x 候选 |

## 3. 主题与配色

- 外壳：Kit Theme/ThemeColor 语义 token；亮/暗/跟随系统；不在 v1 支持 CSS 主题、vibrancy、自定义 CSS。
- 终端：独立 TerminalPalette：
  - 16 色（normal/bright）、foreground、background、cursor、selection。
  - Tabby color scheme 对象直接映射（01 显示配色对象内嵌在 config）；导入时按名字/内容匹配，未命中回退默认。
  - 终端颜色不跟 Kit token 自动变，除非用户选“跟随应用主题”的映射预设；避免 TUI 颜色被 UI 主题意外改掉。
- 字体：外壳用 Kit 默认；终端单独配置 mono 字体/字号/行高/fallback；连字 v1 默认关（与 Tabby 对齐）。

## 4. 可访问性与 IME

- 外壳：Kit AccessKit 角色/名称/状态；键盘 Tab 顺序；菜单/对话框焦点陷阱。
- 终端：AccessKit 暴露文本角色、行/列、选区、光标位置；屏幕阅读器至少能读当前屏；完整终端语义（逐行/逐 cell）放 v1.x。
- IME：GPUI key_char + 平台 IME；候选窗定位到终端光标；三平台手工矩阵：
  - macOS 中文输入法（拼音）预编辑/上屏/取消。
  - Windows 微软拼音 + ConPTY。
  - Linux fcitx5（X11；Wayland 视 GPUI 支持）。
- 验收：v1 smoke（键盘可达、AccessKit 树、IME 三平台手工）；完整审计 v1.x。

## 5. 焦点、滚动、选区

- Workspace 决定活动标签/活动 pane；PaneTree 在 pane 间移动焦点；TerminalPane 拥有自己的 FocusHandle。
- 终端获得焦点时，按键先给终端；AppShell 的快捷键用 GPUI keybinding/命令分发，只在终端不消费/明确 chord 时触发。
- 滚动：TerminalPane 持有 viewport 偏移/scrollback；鼠标滚轮/滚动条/键盘 PageUp/PageDown 走 core；不看 UI 容器的滚动。
- 选区：TerminalPane 持有 anchor/focus/块选择；复制粘贴走宿主；URL 检测/打开由 pane 提供 hover/click。
- 搜索：pane 内的 overlay；高亮由 TerminalElement 绘制。
- 广播输入（multifocus）v1.x。

## 6. 会话恢复

- Workspace 序列化：tabs 顺序、每 tab 的 PaneTree（方向/比例/leaf）、活动 tab/pane、每个 leaf 的 profile id/type、终端标题/自定义标题、窗口大小/位置。
- 不序列化：屏幕内容、scrollback、PTY 句柄（07 已把 PTY re-attach 降 P1）。
- 恢复：启动时重建 PaneTree + 重新 spawn shell/SSH；失败给通知并保留空 pane；配置 schema 向前迁移。
- 不用 Kit DockLayout 作为恢复格式（若 Q1 选自定义拓扑）。

## 7. NSH 边界

- v1：不引入 NSH。理由（02）：
  - NSH 0.4.0 仅数周龄；GPUI 后端走 gpui-fast，而 GPUI Kit 0.9 锁 gpui-pre 0.3.11；混用要整体切 gpui-fast，双框架调试。
  - 终端是自绘密集场景，NSH 的 TSX/声明式 UI 不适合终端网格。
- v1.x：只评估 NSH 做扩展/运行时可下发 UI（14 的扩展面板、AI 生成 UI）；前置 PoC：Kit 全仓切 gpui-fast、NSH 与 Kit 同进程共存、输入/主题/无障碍一致。
- 移动/Web：NSH 可作为未来跨端外壳候选，但不是 v1/v1.x 的承诺；需要独立 proposal。
- 终端 widget 永远不用 NSH。

## 8. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| 自研 PaneTree/TabStrip 工作量大 | v1 外壳延期 | 复用 Kit TabBar/按钮/菜单；先做最小 split + tab，再补拖拽/最大化 |
| Kit 升级/pin 漂移 | API 破坏 | 锁 =0.3.11；封装 Kit 组件边界；季度升级 |
| 终端 AccessKit 语义复杂 | 无障碍不达标 | v1 只做文本/选区冒烟；完整审计 v1.x |
| IME 三平台差异 | 中文输入体验 | 09/11 原型期就手工测；必要时平台 IME 适配 |
| NSH 引入导致双栈 | 进度/稳定性 | v1 不引入；v1.x 先 PoC，失败就放弃 |
| Tabby 落区语义与 GPUI 拖拽差异 | 交互不还原 | 自定义 drop zone overlay；用 09 原型验证拖拽/分屏 |

## 9. 证据

- 02：.scratch/rust-rewrite/research/02-gpui-kit-nsh-capability.md
- 09：.scratch/rust-rewrite/specs/09-prototype-report.md
- 07：.scratch/rust-rewrite/specs/v1-scope.md
- 08：.scratch/rust-rewrite/specs/platform-distribution.md
- Kit 源码：crates/base/src/dock、crates/component/src/setting、crates/component/src/command、crates/component/src/tab、examples/dock。

## 10. 下一步

- 本草案 §0 的 7 项拍板后，12 关闭；16 的 UI 部分用本文件作为输入。
- 11 与 12 可以并行：11 先出 TerminalCore + TerminalElement 最小实现，12 出 Workspace/PaneTree/AppShell 骨架。
