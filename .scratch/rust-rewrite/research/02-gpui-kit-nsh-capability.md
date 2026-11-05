# 02 GPUI / GPUI Kit / NSH 能力评估（research）

- 日期：2026-11-05；上游基准：Eugeny/tabby `467ce8e`（2026-10-06）
- 证据口径：只取一手来源（crates.io / docs.rs / 官方站点 / GitHub 源码 / `/tmp/tabby-recon`）。Zed 源码为 `main` 分支 2026-11-05 抓取，未固定 commit；crates 版本为当日 crates.io 数据。
- 结论先行：**用 GPUI Kit 0.9 作为桌面外壳可以，但终端 widget 必须自研；NSH 不进 v1 主路径。**

## 结论（TL;DR）

1. **GPUI 可用，但没有稳定版 crate。** 官方 `gpui`（crates.io）停在 0.2.2（2025-10-22）；生态实际依赖的是 `gpui-pre` 0.3.11（Zed `main` 快照，周更，发布者是 huacnlee 而非 Zed 官方账号）。GPUI Kit 用 `=0.3.11` 精确 pin。
2. **GPUI Kit 0.9 覆盖"终端应用的外壳"**：Dock 分屏/标签、设置表单、主题、命令面板、通知、表格、虚拟列表、代码编辑器、WebView——但**没有 Terminal 组件**。0.9.0 的 docs.rs 源码树无 terminal 模块，gpui-kit workspace 无 terminal crate，官网 `/component/terminal` 404。
3. **NSH 是另一条产品线**，定位是 TSX/Rust 跨端原生 UI（含 iOS/Android/web/TUI），GPUI 后端以 `AppView` 嵌入；但它依赖 Longbridge 自己的 `gpui-fast` fork（不是 Kit 的 `gpui-pre`），与 Kit 混用要把 Kit 整体切到 `gpui-fast` feature。NSH 首个 crate 发布 2026-10-17，3 周龄，不进 v1。
4. **最大工程风险不是"能力缺不缺"，而是"基座版本漂移 + 终端自研工作量"。** 前者用 fork/vendor + 季度升级窗口吸收；后者应作为独立 epic（见 03/09/11 票）。

## 1. GPUI 版本与三平台成熟度

| 包 | 版本 | 发布日期 | 说明 |
|---|---|---|---|
| `gpui` | 0.2.2 | 2025-10-22 | Zed 官方发布，owners 含 zed-industries |
| `gpui-pre` | 0.3.11 | 2026-11-02 | Zed main 快照（`zed@f816e1f`），周更：0.3.2（09-03）→0.3.11（11-02），发布者 huacnlee |
| `gpui-pre-platform` / `-web` / `-macros` | 0.3.11 | 同上 | 平台层拆包，feature：`x11`/`wayland`/`font-kit`/`runtime_shaders` |
| `gpui-fast` | 0.1.13 | 2026-11-02 | Longbridge 的 GPUI fork（"rendering and layout optimizations"） |

平台实现全部有源码证据，不是文档表态：

| 维度 | macOS | Windows | Linux |
|---|---|---|---|
| 文本 shaping | CoreText（`gpui_apple/Cargo.toml:20,33` 的 `core-text`/`objc2-core-text`） | DirectWrite（`gpui_windows/Cargo.toml:46-53`） | cosmic-text 0.19（`gpui_wgpu/Cargo.toml:26`；`gpui_linux/src/linux/text_system.rs` 直接 re-export `CosmicTextSystem`） |
| 渲染 | Metal（`gpui` 依赖 `objc2-metal`） | D3D11 + DirectComposition | wgpu（Vulkan/GL；`gpui_linux/Cargo.toml` 的 `gpui_wgpu`） |
| IME | `NSTextInputClient`（`gpui_macos/src/window.rs:211`，含 `hasMarkedText`） | `ime_enabled` / `update_ime_position`（`gpui_windows/src/window.rs:59,1066`） | X11 走 `xim`（`gpui_linux/Cargo.toml`），Wayland 侧未见 text-input 协议证据，**需实测** |
| 无障碍 | `accesskit_macos` | `accesskit_windows` | `accesskit_unix`（x11 与 wayland 两个 feature 都启用） |
| 系统通知 | `objc2-user-notifications` + `system_notifications.rs` | 平台层实现 | `notify-rust` |
| 窗口 | 多窗口 `open_window`；`WindowKind::{Normal,PopUp,Menu}`（`crates/gpui/src/platform.rs:422,2839`）；窗口菜单/应用菜单 `set_menus`/`set_dock_menu`（`:494-511`） | 同左 | X11 + Wayland 双栈（`gpui_linux/Cargo.toml:16-45`） |

其它关键能力（同一 `platform.rs`）：

- 剪贴板：文本/图片/`ExternalPaths`，含 async 变体（`:587-609,3143-3151`）。
- 文件拖放：`FileDropEvent::{Entered,Pending,Submit,Exited,Ended}` + `ExternalPaths`（`crates/gpui/src/window.rs:5931-5978`），macOS 实现 `draggingEntered/performDragOperation`（`gpui_macos/src/window.rs:473-489`）。**Tabby 的"拖文件到终端粘路径"可以复刻**。
- 打开文件对话框：`prompt_for_paths`（`:450`）。
- **托盘：没有。** 平台层只有 `ActivationPolicy::Accessory`（"menu bar utility"，`:164-167`），没有 NSStatusItem/Shell_NotifyIcon 封装。
- **全局热键：没有。** Platform trait 里没有 hotkey API；Tabby 用的是 Electron `globalShortcut`（`tabby-electron/src/services/electron.service.ts:17`）。
- **Headless 测试：有。** `Headless` 平台（`:297,333`）、`PlatformHeadlessRenderer`（`:1463-1475`）、`TestAppContext`（`crates/gpui/src/app/test_context.rs`）、`VisualTestPlatform`（`:184`）。

与 Tabby 现状的差距不在渲染，而在"应用级能力"：Tabby 靠 Electron 拿到托盘、全局热键、自动更新、多窗口管理，这些在 GPUI 里是空白或要自建；反过来 GPUI 给了 Tabby 没有的原生 glyph 渲染管线和 AccessKit 语义树。

判断：macOS 最成熟（CoreText + NSTextInputClient + AccessKit + 系统通知全有原生实现），Windows 次之（DirectWrite/IME 齐），Linux 是最弱环——cosmic-text shaping + wgpu + X11/Wayland 双栈，且 IME 只有 X11 有明确实现。Zed 自身三平台在售是"有人在生产用"的证据，但 Linux 的发行版/合成器分布远大于 macOS，验收成本要按"矩阵"算。

## 2. GPUI Kit 0.9 的组件覆盖

版本与节奏：`gpui-component` 0.9.0（2026-11-02，37 个版本，自 2025-02 起）；`gpui-kit` 伞包 0.9.0（2026-09-03 起 14 个版本），周更（0.6.1 09-09 → 0.9.0 11-02）。许可证 Apache-2.0，文档散文 CC BY 4.0（复制需署名）。三层结构：`gpui-kit`（伞）→ `gpui-base`（无样式行为层）+ `gpui-component`（样式层）+ assets；另有可选 `gpui-shell`（JS 扩展运行时，capability 默认拒绝）。

| Tabby 需要的 UI 面 | Kit 0.9 | 证据/备注 |
|---|---|---|
| 标签 / 分屏 / Dock | ✅ DockArea + DockSkin + `DockLayout`（`h_split`/`v_split`/`tabs`/edge docks），`DockAreaState` serde 持久化 | `gpui-kit.com/component/dock` |
| 终端 | ❌ **没有** | docs.rs `gpui_component` 0.9.0 源码树无 terminal；workspace members 无 terminal crate；官网 `/component/terminal` 404 |
| 设置表单 | ✅ Settings（v0.5.0 起）：Settings→SettingPage→SettingGroup→SettingItem→SettingField，可搜索 | `/component/settings` |
| 主题 | ✅ Theme：JSON 颜色 token、明暗、字号/圆角 | `/component/theme` |
| 命令面板 | ✅ Command（过滤命令/快捷操作） | `/component/command` |
| 通知 | ✅ Notification（窗口右上 toast、自动消失）+ SystemNotification（OS 通知，含平台限制说明） | `/component/notification`、`/docs/system-notification` |
| 表格 / 大列表 | ✅ Table/DataTable（虚拟滚动、排序、列管理）、VirtualList | README |
| 代码编辑器 / 富文本 | ✅ CodeEditor（tree-sitter、LSP、200K 行）、Markdown、HTML、Chart | README |
| WebView | ✅ `gpui-webview` 0.9.0（基于 Wry） | crates.io |
| WASM | ⚠️ 能构建，但官方明说"主要用于 showcase，未验证为完整应用的分发路径"；单 canvas/单窗口、无字体库、CJK 靠子集或 canvas fallback | `/docs/webassembly` |
| UI 测试 | ✅ `#[gpui_kit::test]` + TestAppContext：headless window、真实事件、AccessKit 断言、离屏像素 | `/docs/test`、`/docs/accessibility` |

组件数量口径有出入（crate 描述 60+、README 75+），但结论不受影响：Kit 覆盖的是"表单 / 展示 / 布局 / 编辑"，没有 terminal、multiplexer 这类领域组件。Kit 的 Dock 是布局系统，不是"终端分屏"——把 PTY 会话塞进 panel 时，session 生命周期、焦点切换、关闭确认都要自己做。

关键缺口排序：**terminal（工作量最大）> 托盘 > 全局热键 > Linux IME 细节 > 原生窗口层级/浮动面板（Kit 有 popup/menu 窗口，但不是任意浮动面板）**。

## 3. NSH：定位、嵌入与成熟度

- 定位（nsh.dev）：一套 Rust engine + Taffy 布局 + reactive graph，TSX 或 Rust 写 UI，各平台用**平台原生控件**绘制（iOS UIKit、Android View、AppKit、Win32/D2D、GTK4、GPUI、ratatui、HTML）。核心卖点：运行时可下发 TSX 模块、签名包、host 配置 capability 沙箱、AI 生成 UI。
- GPUI 后端嵌入：`nsh-platform-gpui` 的 `AppView` 就是一个 GPUI view（`Entity<AppView>`），放进 Kit 的 `Root`/panel/dock 任意位置；`AppView::new(AppOptions::default(), window, cx)` + `view.load(App::Rust(Root::of::<App>()))` 或 `App::Js(Bundle::source(code))`；支持 host channel、limits/sandbox、dev server 热更新（`nsh.dev/docs/gpui`）。
- **但基座不是同一个 GPUI**：`nsh-platform-gpui` 0.4.0 的依赖是 `gpui-fast ^0.1.9` + `gpui-fast-platform`，而 Kit 0.9 默认 pin `gpui-pre =0.3.11`。两者 lib 名都叫 `gpui`，同一进程只能留一个；要用 NSH 就必须给 Kit 开 `gpui-fast` feature（0.9.0 features 里确实有，会整体切到 gpui-fast 0.1.13）。GPUI Kit README 里"NSH 的 GPUI 后端 will be built on GPUI Kit"这句话与 crate 依赖不一致，属于文档滞后，不能当集成保证。
- 许可证：Apache-2.0（README）。成熟度：`nsh-core` 首版 0.1.0 于 2026-10-17，最新 0.4.0 于 2026-11-04；下载量 nsh-core 2k、nsh-platform-web 294、nsh-platform-gpui 385、nsh-gpui-host 42。3 周龄 + 个位数下载量 = 早期项目。
- NSH 的 TSX 跑在 QuickJS 上（无 JIT），对"插件面板"够用，对"终端渲染"不构成方案。它的 signed package + capability 模型与 14 票（扩展机制）方向一致，更适合作为那张票的输入，而不是 UI 主路径。

对终端场景的适配：NSH 的抽象是"声明式 UI + 平台原生控件"，而终端是"cell grid + 自绘 glyph + 极低输入延迟"，NSH 的 GPUI 后端文档自己列了一串绘制差异（旋转文本不转、圆角裁剪不吃文字、canvas 文本近似、blur/backdropFilter 不画）。**不适合作为终端渲染层**；适合作为"扩展面板 / AI 生成 UI 的运行时"。

## 4. 能做 / 不能做 / 需自研

| 分类 | 内容 | 代价 |
|---|---|---|
| Kit 直接能做 | Dock 分屏与标签、可拖拽 tab、布局持久化；设置页（分组+搜索+多种 field）；主题/明暗；命令面板；toast + OS 通知；表格/虚拟列表；代码编辑器；Markdown/HTML；AccessKit 语义；headless UI 测试 | 低；接入成本主要在"把 Tabby 的信息架构映射到 Kit 组件" |
| 需少量适配 | 多窗口（`open_window` + WindowKind）、应用菜单/窗口菜单、文件拖放、文件选择框、剪贴板多格式、系统通知 | 中低；注意 macOS 菜单栏与 Windows 菜单模型差异 |
| **必须自研** | ① 终端 widget（PTY/VT 输出 → grid → GPU glyph/quad 绘制、选区、滚动、URL 检测、搜索高亮）；② 托盘（NSStatusItem / Shell_NotifyIcon / StatusNotifierItem）；③ 全局热键（`global-hotkey` 类 crate + 主线程事件桥）；④ 分屏的拖拽落区交互（Kit Dock 提供结构，Tabby 的 drop zone 语义要自己接）；⑤ 会话恢复的窗口/标签拓扑（Kit 只存 dock 布局） | ① 是数量级最大的一项；②③ 是平台胶水，各 3–5 人日量级 |
| 不该做 | 用 NSH 画终端；v1 做 WASM/Web 版；依赖未发布的 "Terminal builder" 组件 | 见风险 R6/R7 |

关于终端自研的可行路径：Zed 自己的 `crates/terminal` 用 `alacritty_terminal` + `vte`（`crates/terminal/Cargo.toml:24,46`，Apache-2.0），`terminal_view` 直接在 GPUI 里画 glyph——这条路径已被 Zed 验证，可作为 Rabby 终端 widget 的参照实现（不是直接依赖，Zed 的 view 层耦合 workspace/editor）。

## 5. 三种 UI 组合取舍

| 组合 | 内容 | 优点 | 代价/风险 | 适配度 |
|---|---|---|---|---|
| **A. GPUI Kit + 自研 terminal（推荐）** | Kit 提供 dock/settings/theme/command/notification；终端自己写 | 外壳 3–4 个月省掉；与 Kit 的 Dock/settings 语义天然对齐；单 backend（gpui-pre） | 绑 `=0.3.11` 周更快照；terminal 是全新代码 | ★★★★★ v1 |
| B. 裸 GPUI + 全自研组件 | 只用 `gpui`，不用 Kit | 控制力最强、依赖最少 | 要重写 dock/tab/settings/theme/command/notification/table ≈ 数人年；且仍受 GPUI 版本漂移影响 | ★★☆☆☆ 不推荐 |
| C. Kit + NSH 面板（混合） | Kit 外壳 + NSH `AppView` 承载插件/AI 生成 UI | 扩展生态与"运行时可下发 TSX"是差异化；NSH 已有 sandbox/签名包 | 需把 Kit 整体切到 `gpui-fast`；双框架调试、双份 GPUI 认知；NSH 3 周龄 | ★★★☆☆ v1.x 可选 |
| D. 非 GPUI（egui/iced/Qt/Slint） | 换基座 | 生态成熟、文档多 | 无 dock/命令面板/主题体系，终端 GPU 文本控制力弱；与"Tabby 级 UI"差距大 | ★☆☆☆☆ 仅作 fallback |

推荐：**A 作为 v1；C 只作为 v1.x 的"扩展 UI 运行时"候选**，且引入前必须先做 `gpui-fast` 全仓切换 PoC。B/D 只在 A 的关键假设被推翻时启用。

选 A 的核心理由是风险集中度：把不确定性收在"一个可替换的 widget（terminal）+ 一个可 fork 的基座（gpui-pre）"上，而不是分散到十几个自研组件；C 则是把两套 GPUI 版本风险叠在一起。对应的验收建议写进 07 票：三平台 CJK/IME 冒烟、Linux 合成器矩阵（X11 + Wayland 各一）、Dock 布局恢复，以及"关窗常驻"缺托盘时的降级行为。

## 6. 风险清单（含触发条件与缓解）

| # | 风险 | 触发条件 | 缓解 |
|---|---|---|---|
| R1 | GPUI 版本漂移 + 第三方 pin | `gpui-pre` 周更；发布者非 Zed 官方；Kit 用 `=` 死锁单版本 | fork + `cargo vendor` 锁定；季度升级窗口；升级前跑 Kit 的 UI 测试套件 |
| R2 | terminal 自研低估 | 有人把"Kit 有 Terminal builder"当既有能力 | 已在票面纠正：0.9 无 terminal；按独立 epic 排期，先做 09 的垂直切片 PoC |
| R3 | Linux 弱环 | cosmic-text shaping / wgpu / Wayland-X11 组合问题；IME 仅 X11 有明确实现 | CI 增加 Ubuntu X11+Wayland 冒烟；CJK/IME 在 Linux 单列验收项 |
| R4 | 托盘/全局热键缺失 | 产品要求"关窗常驻/一键唤起" | 自研平台胶水（`tray-icon`/`global-hotkey`）；v1 可先不做托盘 |
| R5 | IME/CJK 体验 | 中文输入法候选窗定位、emoji、双宽字符、组合输入 | 早期做 CJK 冒烟用例（三个平台各一套）；Kit 的 Input 已有 AccessKit/测试基础 |
| R6 | WASM 不成熟 | 有人提"顺手出 Web 版" | 官方已声明仅 showcase；v1 明确不做 |
| R7 | NSH 与 Kit 双 GPUI 栈 | 直接 `cargo add nsh-platform-gpui` 到 Kit 应用 | 必须开 Kit `gpui-fast` feature；先 PoC，再决定 |
| R8 | 许可证/归属 | 复制 Kit 文档散文（CC BY 4.0 需署名）；gpui-pre 快照需保留 Zed notices | 文档引用改为链接；NOTICE 文件保留上游许可 |

## 7. 未决与证据不足

- GPUI Kit 在 Windows/Linux 的生产验证强度：Longbridge Pro 的平台分布未公开，不能从"macOS 上很成熟"推三平台。
- NSH GPUI 后端与 Kit 0.9 的实测兼容：官方无集成示例；README 与 crate 依赖互相矛盾。
- 托盘/全局热键是否有可用的 GPUI 社区封装：未找到一手 crate 证据，按"自研"估算。
- Tabby 的多段 keystroke（`hotkeys.service.ts` 的 `PastKeystroke`/`matchedLength`）能否用 GPUI `KeyBinding` 等价表达：未验证，属 12 票（UI 架构）范围。
- 终端 grid 走 GPUI glyph atlas 的吞吐/延迟：需要 09 票的原型给数，不在此票下结论。

## 参考（一手来源）

- crates.io：`gpui` 0.2.2、`gpui-pre` 0.3.11、`gpui-kit`/`gpui-component`/`gpui-base`/`gpui-fast` 0.9.0/0.1.13、`nsh-*` 0.4.0（含 owners、feature、dependencies 接口）
- Zed 源码（`main`，2026-11-05）：`crates/gpui/src/platform.rs`、`crates/gpui/src/window.rs`、`crates/gpui/src/app/test_context.rs`、`crates/gpui_wgpu/Cargo.toml`、`crates/gpui_linux/Cargo.toml`、`crates/gpui_macos/Cargo.toml` + `src/window.rs`、`crates/gpui_windows/Cargo.toml` + `src/window.rs`、`crates/gpui_apple/Cargo.toml`、`crates/terminal/Cargo.toml`
- GPUI Kit：`github.com/longbridge/gpui-kit`（README、workspace `Cargo.toml`）、`gpui-kit.com/component/{dock,settings,theme,command,notification}`、`/docs/{accessibility,test,webassembly,system-notification}`
- NSH：`nsh.dev/docs/{gpui,platforms}`、`github.com/longbridge/nsh`（README、license）
- Tabby（`467ce8e`）：`tabby-terminal/src/frontends/xtermFrontend.ts:14,854`（WebglAddon）、`tabby-core/src/components/splitTab.component.ts`、`tabby-core/src/services/hotkeys.service.ts`、`tabby-electron/src/services/electron.service.ts:17`、`package.json:38`（electron 43）
