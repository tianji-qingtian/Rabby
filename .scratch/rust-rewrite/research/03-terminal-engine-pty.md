# 03 研究：Rust 终端引擎与 PTY 选型

- 票号：03（`wayfinder:research`），父图 `.scratch/rust-rewrite/map.md`
- 上游基准：Eugeny/tabby @ `467ce8e`（2026-10-06，commit "xterm 6"）
- 核对时间：2026-11-05。wezterm / alacritty 取仓库 `main` / `master`；版本与下载量取当日 crates.io API
- 边界：本笔记只供决策与 grill 使用；"内核/渲染最终取舍 + ADR"归 11（`terminal-内核与渲染边界`，blocked by 03/09）

## 结论（TL;DR）

1. **内核：选 wezterm-term**（以 git 依赖 pin commit，先用不改；需要打补丁时再 fork），置信度中高。它是唯一"开箱即用"覆盖 Tabby 默认行为的 Rust 内核：SIXEL / iTerm2 / kitty graphics、OSC 8 超链接、OSC 52 剪贴板、OSC 9;4 进度、DECSET 2026 同步输出、resize rewrap、脏行（dirty line）与 SequenceNo、按 grapheme 存格的 Cell。代价是它**没有发布到 crates.io**，API 是 wezterm 内部 API（0.1.0，无 semver 承诺），依赖面大。
2. **若 11 决定 v1 不做图片协议 + bidi**，则改选 **alacritty_terminal 0.26**（Apache-2.0、已发布、有 damage tracking 与 regex search），这是本笔记唯一实质分叉点。
3. **vte / termwiz / vt100 只做候选说明，不建议承载 v1**：vte 只是解析器（屏幕模型要自写），termwiz 是 wezterm 旧一代模型库（最后发布 2025-03），vt100 屏幕模型过简。自研等价的 VT 覆盖 = 3~6 个月，且 Tabby 兼容性无从验证。
4. **PTY：portable-pty 0.9.0**（MIT，就是 wezterm 的 `pty` crate）：Unix 走 `openpty` + `setsid` + `TIOCSCTTY`，Windows 动态加载 ConPTY，同一套 `MasterPty/Child` API。**pty-process 只覆盖 Unix**（rustix + tokio），Windows 会分叉出第二套代码，不值得；**conpty crate 停止维护（2024-09）**，不要直接用。
5. **GPUI Kit 没有 Terminal 组件**（0.9.0 的 11 个 crate 里没有终端源文件），所以网格渲染必须自绘：`shape_line` + `Quad` + 字形 atlas（`MonochromeSprite`/`PolychromeSprite`）+ `paint_image` 画图；dirty-row 只能"模型侧 damage 驱动重建行数据 + view 级重绘"，GPUI 无行级失效。

## Tabby 现状基线（要平齐的行为面）

| 维度 | Tabby 现状 | 证据 |
|---|---|---|
| 终端前端 | xterm.js 6.0.0 + fit/image/ligatures/search/serialize/unicode11/webgl addon | `tabby-terminal/package.json` devDeps |
| PTY | node-pty `^1.2.0-beta.8`（ConPTY / winpty 双后端） | `app/package.json:34`、`tabby-terminal/src/frontends/xtermFrontend.ts:150-160` |
| scrollback | 25000 行（配置项），写入 `xterm.options.scrollback` | `tabby-terminal/src/config.ts:60`、`xtermFrontend.ts:734` |
| 图片 | `sixel: true` 默认开，加载 ImageAddon（SIXEL + iTerm2 IIP） | `config.ts:62`、`xtermFrontend.ts:210-212` |
| 连字 | `ligatures: false` 默认关，且 Web 平台不加载 | `config.ts:29`、`xtermFrontend.ts:748-750` |
| 进度 | `detectProgress: true`（OSC 9;4 类） | `config.ts:59` |
| bracketed paste | `bracketedPaste: true`，读 `modes.bracketedPasteMode` | `config.ts:27`、`xtermFrontend.ts:815` |
| 对比度/粗体 | `minimumContrastRatio: 4`、`drawBoldTextInBrightColors: true` | `config.ts:61-63`、`xtermFrontend.ts:736-739` |
| OSC 处理 | 中间件自己拦 OSC 1337（CurrentDir）与 OSC 52（写剪贴板），其余透传 | `tabby-terminal/src/middleware/oscProcessing.ts:66-84` |
| 流控 | 自建 FlowControl：128KB 阈值 + 高/低水位 10/5，xterm.write 回调做背压 | `xtermFrontend.ts:55-112` |
| 未显示会话缓冲 | 8MB 上限，超出丢最旧整块 | `tabby-terminal/src/session.ts:8,44-50` |
| ConPTY 判定 | 构建号 19041 起支持、21376 起支持 reflow；不满足时给 xterm `windowsPty` 降级 | `tabby-core/src/utils.ts:7-8`、`xtermFrontend.ts:154-159` |

## 内核候选对比

| 维度 | wezterm-term | alacritty_terminal 0.26 | vte 0.15 | termwiz 0.23 / vt100 0.18 |
|---|---|---|---|---|
| 定位 | 完整 VT 模型（含输入编码） | 完整 VT 模型 | 纯解析器 | 旧一代模型 / 简易模型 |
| 发布 | **未上 crates.io**（API 返回 "crate does not exist"）；第三方转发 `zeughaus-wezterm-term` 0.1.0（2026-09-26，182 次下载）、`tattoy-wezterm-term` fork（2025-07-11，18k） | 0.26.0（2026-04-06），2.48M 下载 | 0.15.0（2025-02-02），92M 下载 | 0.23.3（2025-03-20）/ 0.18.1（2026-10-19） |
| 许可 | MIT | Apache-2.0 | Apache-2.0 OR MIT | MIT |
| 屏幕模型 | Screen + VecDeque 行 + scrollback，resize 时 rewrap | Grid 扁平存储 + 行 damage | 无 | 有（弱） |
| 宽字符/emoji | Cell 存 `TeenyString`（整簇），配 finl_unicode + NFC 归一化 + bidi | Cell 是 `char` + `CellExtra`（zero-width 上限 9 个 char） | 无 | 弱 |
| SIXEL / iTerm2 / kitty 图片 | 三者都有 | **无** | 无 | 无 |
| OSC 8 / OSC 52 | 有（Clipboard trait） | 有（事件回调） | 无 | 部分 |
| OSC 9;4 进度 | 有（Progress 告警） | 无 | 无 | 无 |
| 同步输出 DECSET 2026 | 有 | 有（TermMode） | 无 | 无 |
| search | **无**（GUI 层另做） | 有（regex / 语义 / 括号 / inline） | 无 | 无 |
| 序列化 | serde feature | serde feature + RenderableContent | — | — |
| 输入编码 | 有（key/mouse 编码 + kitty keyboard 开关） | 有（kitty keyboard） | 无 | 部分 |
| 维护活跃度 | wezterm 主仓活跃，但 term 相关 crate 自 2025-03 起未再发布 | alacritty 0.18.0-dev 活跃，库随版本发 | 稳定低频 | termwiz 低频 |

要点（有证据的差异，不做泛泛比较）：

- wezterm-term 的自我定位写得很清楚：解析、键盘/鼠标输入编码、scrollback、**sixel 与 iTerm2 图片**、OSC 8 超链接；**不含 GUI、不管理 PTY**，入口是 `advance_bytes`（`wezterm/term/src/lib.rs:1-15`、`term/src/terminal.rs:164`）。宿主通过 `TerminalConfiguration` 注入策略：scrollback 大小、CSI-u、调色板、粘贴换行规范化、kitty graphics/keyboard 开关、Unicode 版本、NFC 归一化、bidi 模式（`term/src/config.rs:146-219`）。
- wezterm-term 的 Cell 是 `text: TeenyString + attrs`（`wezterm-cell/src/lib.rs:719-729`），意味着一个格能存整个 grapheme cluster；alacritty 的 `Cell` 是 `c: char + fg/bg/flags + Option<Arc<CellExtra>>`，zero-width 字符封顶 `MAX_ZEROWIDTH_CHARS = 9`（`alacritty_terminal/src/term/cell.rs:17,132-147`）。对 CJK + emoji ZWJ + 组合字符，这是结构性差异，不是补丁能补的。
- alacritty 的强项在"渲染友好"：行级 damage（`term/mod.rs:141-198`）、`renderable_content()` 快照（`:637`、`:2393`）、regex 搜索（`term/search.rs:34,121,472`）、reflow（`grid/resize.rs:14,101`）。wezterm 的脏行是 `Screen::dirty_line` + SequenceNo（`term/src/screen.rs:340`），够用但不是给外部渲染器设计的 API。
- 图片/进度是分水岭：wezterm 有 `iterm.rs` / `sixel.rs` / `kitty.rs` 三个实现，OSC 9;4 走 `ConEmuProgress` → `Progress`（`term/src/terminalstate/performer.rs:1082-1088`），剪贴板走 `SetSelection/QuerySelection` + `Clipboard` trait（`terminalstate/mod.rs:788-800`、`terminal.rs:13-19`）。alacritty 侧完全无图片协议；OSC 52 以事件形式交给宿主（`term/mod.rs:1705,1726`，事件见 `event.rs:24-31`），安全策略因此天然在宿主手里。
- xterm.js 的 search/serialize 是 addon；换到 Rust 后无论选谁都要自研"屏幕 → ANSI 快照"的序列化，以及跨 scrollback 的搜索（wezterm 无 search，alacritty 有基础设施但要接 UI）。

## PTY 层对比

| 维度 | portable-pty 0.9.0 | pty-process 0.5.4 | 直接 ConPTY |
|---|---|---|---|
| 许可 / 维护 | MIT，wezterm 主仓，2025-02 发布 | MIT，2026-10-19 发布，活跃 | 系统 API，自己封装 |
| Unix | openpty + fork/exec，`setsid` + `TIOCSCTTY` 建立控制终端 | 分配 pty 并作为控制终端，tokio/阻塞双形态 | — |
| Windows | 动态加载 `CreatePseudoConsole/ResizePseudoConsole/ClosePseudoConsole`；**没有 winpty 回退** | **无实现**（依赖只有 rustix，文档只讲 pts） | 需自己处理句柄继承、进程属性、resize 时序 |
| API 面 | `PtySize`、`MasterPty{resize,get_size,try_clone_reader,take_writer,process_group_leader,as_raw_fd,tty_name,get_termios}`、`Child/ChildKiller`、`CommandBuilder{cwd,env}` | `Pty` + `Command`，返回标准 `Child` | 自行设计 |
| 进程树 | 只有 `TerminateProcess`（Windows）/ kill（Unix），**无 Job Object** | 同左（Unix 进程组） | 可自建 Job Object |
| 其他 | 附带 `serial.rs`（serial2），对串口插件有用 | 无 | — |

要点：

- portable-pty 的 trait 形状（`wezterm/pty/src/lib.rs:65-172,265-269`）足够 Rabby 用：读端可 clone、写端可取走、尺寸可查可改、能拿 `tty_name`/`termios`（shell integration、通知、进程工作目录都能接）。
- Windows 的坑不在 API 而在语义：ConPTY 在 21376 之前不 reflow，且 resize 后 scrollback 行为与 xterm 期望不同——xterm.js 自己用 `windowsPty` 里的 backend/buildNumber 打开补偿（`xterm.d.ts:324-338`），Tabby 照抄成配置（`xtermFrontend.ts:154-159`）。Rust 内核若不做同样的补偿，Windows 上 resize 会丢行。
- portable-pty 在 `pty/src/win/` 下没有 Job Object 相关代码（grep 无命中），关窗杀整棵进程树需要自己加一层；这是 Windows 上"关标签但残留子进程"的典型来源。
- pty-process 的 docs.rs 最新版构建失败（0.5.4），且没有 Windows 实现——作为跨三平台产品的基础设施，它只能当 Unix 侧的备选。
- conpty crate 最后发布 2024-09-23，已不建议新项目采用。

## 与 GPUI 的集成

- **没有现成组件**：gpui-kit 0.9.0（Apache-2.0，2026-11-02）的 crate 列表是 assets / base / component / component-macros / component-shell / fps / kit / shell / story / story-web / webview，仓库中没有终端组件（仅有 terminal 图标）。终端视图是自研 Element。
- **字体度量**：`Window::text_system()`（`gpui/src/window.rs:2677`）拿到 `TextSystem`，用 `layout_width(font_id, font_size, ch)` 算单元格宽度（`text_system.rs:416,904`），行高用同字号 ascent/descent；网格几何 = 列宽 × 行高，必须先由真实字体度量得出，不能假设 8px 格。
- **行 shaping**：`shape_line(text, font_size, runs, force_width)`（`text_system.rs:672`）一次塑形一整行，`TextRun.features: FontFeatures`（`text_system.rs:1141`，默认空 `1163`）可按 run 打开连字。**连字与等宽网格天然冲突**：连字会改变字形宽度，导致"第 N 列 ≠ N × 列宽"。建议 v1 与 Tabby 保持一致默认关（`config.ts:29`），只在同色同属性的连续文本段内试开，并用原型验证列对齐。
- **绘制**：`Primitive::{Quad, MonochromeSprite, PolychromeSprite, Surface}`（`gpui/src/scene.rs:222-231`）；背景/光标/下划线用 `paint_quad`（`window.rs:4934`）、文字走 glyph atlas 的 `paint_glyph`（`:5071`）、SIXEL/iTerm2 图片用 `paint_image`（`:5303`，配 `drop_image`/`has_image_atlas_entry` 管理生命周期）。自定义 Element 只需实现 `request_layout/paint`（`element.rs:96`）。
- **dirty-row**：GPUI 只有 view 级失效 `invalidate_view`（`window.rs:171`）+ 整帧重绘，没有矩形/行级失效。所以策略是：终端模型给出脏行（wezterm `dirty_line`+`SequenceNo`，alacritty `LineDamageBounds`）→ 只重建脏行的 `SharedString`/`TextRun` 缓存 → `cx.notify()`。重绘成本靠 glyph atlas 复用摊薄，但要接受"整窗重绘"这一事实，别指望 GPUI 帮你省 draw call。
- **IME**：`EntityInputHandler`（`gpui/src/input.rs:26`）的 `bounds_for_range`（`:81,239`）决定候选窗位置，`replace_and_mark_text_in_range`（`:71`）处理合成串，`text_input_configuration`（`:132`）控制输入法行为。preedit 必须在网格里自绘（xterm.js 用隐藏 textarea，GPUI 没有等价物）。Tabby 为 fcitx5 中文标点与 Windows 搜狗 Shift 各打了一个补丁（`xtermFrontend.ts:32-51,166-179`），说明 IME 是兼容性高发区，必须在 ticket 09 原型里覆盖。

## xterm.js 6 行为差异清单（会破坏兼容的点）

| 项 | xterm.js 6 契约 | Rust 侧要做什么 |
|---|---|---|
| kitty keyboard | `vtExtensions.kittyKeyboard` **默认 false**（opt-in） | wezterm/alacritty 默认支持 kitty keyboard，必须显式关到同一默认，否则 vim/neovim 的键位行为不同 |
| win32-input-mode | `vtExtensions.win32InputMode` 默认 false（DECSET 9001） | 同上，默认关；仅在明确需要 ConPTY 输入保真时开 |
| 配色查询 | `colorSchemeQuery` 默认 true（CSI ? 996n / DECSET 2031） | 需确认内核是否实现；不实现时 TUI 主题切换行为不同 |
| 光标闪烁 | DECSET 12 由 `quirks.allowSetCursorBlink` 控制，默认关 | 渲染层自己吞掉 12，别让应用改光标闪烁 |
| ConPTY 补偿 | `windowsPty` 存在即改滚动行为；`buildNumber < 21376` 或非 conpty 则关 reflow、按"行尾非空白 = 软换行"推断 | 内核+PTY 层都要带同一套判定，否则 Windows resize 丢行 |
| bracketed paste | 核心解析 DECSET 2004；`ignoreBracketedPasteMode` 可强关 | 粘贴编码（加/不加 `\x1b[200~\x1b[201~`）由宿主实现，Tabby 默认开 |
| OSC 52 | xterm 不内建写剪贴板；Tabby 自己在中间件里拦（`oscProcessing.ts:77-81`） | 是否允许写剪贴板 = 安全策略，必须在宿主层可关 |
| OSC 1337 | Tabby 只认 `CurrentDir=`（`oscProcessing.ts:66-76`） | iTerm2 私有 OSC 需内核暴露或宿主前置解析 |
| OSC 8 | xterm 需要 `linkHandler`（`xterm.d.ts:153,1479`）且默认 confirm | 两个内核都支持 hyperlink，但点击确认/打开策略要自己实现 |
| search / serialize | addon（`addon-search`、`addon-serialize`） | 自研：屏幕搜索 + 屏幕→ANSI 快照；wezterm 无 search，alacritty 有基础设施 |
| 图片 | `addon-image`（SIXEL + iTerm IIP），pixelLimit 默认 16M 像素 | 自研解码 + GPUI 纹理；大图要限流，Tabby 没配 pixelLimit |
| 宽字符 | `addon-unicode11` 切 Unicode 11 宽度表 | wezterm 有 `unicode_version` 配置项；alacritty 用 unicode-width，语义不同 |
| 渲染级选项 | `minimumContrastRatio`、`drawBoldTextInBrightColors` | 自研渲染层实现（xterm 内建，两个 Rust 内核不管渲染） |

## 风险

1. **wezterm-term 的发布形态是最大风险**：crates.io 无此 crate，只能 git 依赖或 vendor；0.1.0、edition 2018、`TerminalConfiguration` 这种"宿主实现一堆 trait"的形态，都不是给外部产品设计的 API。缓解：自建 `TerminalCore` façade（输入字节/键鼠事件入、屏幕快照/脏行/告警出），内核可替换；pin commit，季度 rebase。
2. **依赖图与编译时间**：termwiz、image、lru、miniz_oxide、wezterm-dynamic、wezterm-bidi、terminfo 等（`term/Cargo.toml`）。对启动/包体敏感的话要在原型里量。
3. **性能/内存无公开基准**：本票只拿到结构性事实（wezterm 按 `rows + scrollback` 预分配行容器，`screen.rs:73`；alacritty 扁平存储按 1000 行块扩容，`grid/storage.rs:13`）。吞吐与内存必须由 ticket 09 原型实测，**不得引用二手数字**。
4. **连字 × 网格 × emoji**：三者叠加时列对齐、彩色字形 fallback、ZWJ 序列的簇边界都要在 GPUI 里验证；GPUI 的 `PolychromeSprite` 存在，但终端语义下的 emoji 宽度由内核决定，渲染层只能跟随。
5. **IME 跨平台**：macOS/Windows/Linux 三套输入法行为差异大，Tabby 的两个补丁是现成预警。
6. **ConPTY 老版本**：Windows 10 1809~19044 的 reflow/滚动差异会直接体现为"resize 后历史错位"，需要按 buildNumber 分支。

## 推荐（给 11 的输入）

**决策草案**：终端内核 = wezterm-term（git pin）+ 自建 `TerminalCore` 边界；PTY = portable-pty + 自建 Windows Job Object 收尾；渲染 = GPUI 自绘网格，v1 默认关连字、开图片（与 Tabby 对齐）。

**唯一前置判断题**：v1 是否保留 SIXEL/iTerm2 图片与 bidi？
- 保留 → wezterm-term（省掉 image/bidi 的全部工作量）。
- 不保留 → alacritty_terminal 更省心（Apache-2.0、已发布、damage+search 现成、Cell 语义局限在图片场景外影响可控）。

**ticket 09 原型需要产出的四个数/四个结论**：
1. `yes` / `cat` 大文件吞吐（MB/s）与 25k scrollback 内存占用（wezterm-term vs 必要时 alacritty）；
2. portable-pty 下 zsh/pwsh 的 resize、SIGWINCH、ConPTY reflow 实测；
3. GPUI 80×24 网格 + 1 个 emoji + 连续文本的帧时间，以及脏行重建的 CPU 占比；
4. vttest / esctest 子集对 xterm.js 6 的差异清单（至少覆盖上表 14 项）。

置信度：内核选型 **中高**（证据充分，风险集中在发布形态）；PTY 选型 **高**；GPUI 集成方案 **中**（API 已核实，性能与 IME 未实测）。

## 一手证据索引

- Tabby：`app/package.json:34`；`tabby-terminal/package.json`；`tabby-terminal/src/config.ts:20-63`；`frontends/xtermFrontend.ts:55-112,150-160,210-212,734,748-750,815`；`middleware/oscProcessing.ts:66-84`；`session.ts:8,44-50`；`tabby-core/src/utils.ts:7-8`
- wezterm：`term/src/lib.rs:1-15`、`term/src/terminal.rs:13-19,164`、`term/src/config.rs:146-219`、`term/src/screen.rs:73,101,194,340`、`term/src/terminalstate/mod.rs:360,622,788-800,1300,1694-1704,1783-1789`、`term/src/terminalstate/performer.rs:1082-1088`、`wezterm-cell/src/lib.rs:719-729`、`pty/src/lib.rs:65-172,265-269`、`pty/src/unix.rs:22,228,257,271`、`pty/src/win/pseudocon.rs:32-40`、`pty/src/win/conpty.rs:54,85-87`、`pty/src/cmdbuilder.rs:315,358`
- alacritty：`alacritty_terminal/src/term/mod.rs:141-198,637,1705,1726,1874,2393`、`term/cell.rs:17,132-147`、`term/search.rs:34,121,472`、`grid/resize.rs:14,101`、`grid/storage.rs:13`、`Cargo.toml`
- GPUI / GPUI Kit：`crates/gpui/src/text_system.rs:416,672,904,1141,1163`、`window.rs:171,2677,4934,5071,5303`、`scene.rs:222-231`、`element.rs:96`、`input.rs:26,71,81,132`；gpui-kit crate 列表（jsdelivr 仓库树）
- xterm.js 6：`typings/xterm.d.ts:136,153,313,324-338,451-508,970,1479,2083`；`@xterm/xterm@6.0.0` package.json；`@xterm/addon-image@0.9.0` README
- 版本/许可：crates.io API（wezterm-term 返回 "crate does not exist"；alacritty_terminal 0.26.0；vte 0.15.0；portable-pty 0.9.0；pty-process 0.5.4；termwiz 0.23.3；gpui 0.2.2；gpui-kit 0.9.0）
