# Rabby 终端内核与渲染边界（草案 v0.1）

> 状态：11 票「终端内核与渲染边界」的决议，2026-11-05。用户已确认 §0 的 Q1–Q7 全部按推荐 A，本文作为 v1 内核/渲染基线。
> 输入：03（内核/PTY 选型）、09（垂直切片实测）、07（v1 验收）、12（UI 架构）；wezterm-term 源码 e68a8ec 的 TerminalState/Screen API。
> 一句话：v1 用 wezterm-term（git pin）+ 自建 TerminalCore façade + GPUI 自绘 TerminalElement；UI 线程解析 + 字节预算；不做 fork，不换内核。

## 0. 已拍板（2026-11-05：Q1–Q7 全部按推荐 A）

> 决议：Q1=A wezterm-term git pin + TerminalCore façade，不 fork；Q2=A v1 UI 线程解析 + 256KB/tick 字节预算，worker v1.x；Q3=A 自研 TerminalElement（shape_line + paint_quad + 行缓存 + 脏行 diff）；Q4=A core headless，UI 拥有选区/滚动/搜索；Q5=A 14 项 P0 + vttest/esctest 子集 + golden，图片/bidi/搜索/序列化 P1；Q6=A 07 门槛 + 09 基线 + criterion/gpui-fps；Q7=A key_down/up + mouse_event + send_paste + focus_changed，kitty/win32 默认关。

| # | 问题 | 推荐 | 备选与代价 |
|---|---|---|---|
| Q1 | 内核与 fork | A. wezterm-term git pin（e68a8ec）+ 自建 TerminalCore façade，暂不 fork；只有上游 API 挡住关键能力时才 fork | B. 现在就 fork wezterm-term：维护成本高，v1 不需要；C. 换 alacritty_terminal：只有 v1 砍掉图片与 bidi 才值得，07 已把六格图片放 P1、v1 仍需内核支持 |
| Q2 | 解析线程 | A. v1 在 UI 线程解析，按字节预算（目标 256KB/tick，约 10MB/s@60fps）；TerminalCore 保持同步 façade；若 09 目标不达标，v1.x 把 core 移到 worker，façade 不变 | B. 第一天就做 parser worker + 快照/damage 通道：吞吐更好，但输入顺序、快照成本、生命周期更复杂；C. 独立进程：v1 过重 |
| Q3 | 渲染方式 | A. 自研 GPUI TerminalElement：每脏行 shape_line + TextRun（颜色/粗斜/下划线），背景/选区/光标用 paint_quad；行缓存按 SequenceNo 失效；不用 div 文本行 | B. 沿用 09 的 div 行布局：实现快，但 36 行在 32MB feed 下约 30fps，无法做逐 cell 背景/选区；C. 逐 cell glyph quad：连字/宽字符/emoji 代价过高 |
| Q4 | core/UI 边界 | A. core 只做 headless VT：feed/resize/key/mouse/paste/focus + 屏幕快照/damage + title/cwd/progress + 剪贴板事件；UI 拥有选区、滚动 viewport、搜索 UI、URL 检测；wezterm 类型不外泄 | B. core 也管选区/滚动：UI 难做多种选择语义；C. 直接暴露 wezterm 类型：UI 与 0.1.0 API 耦合 |
| Q5 | 兼容范围 | A. v1 做 03 的 14 项 P0 契约 + vttest/esctest 子集 + golden 快照；图片/双向文本/搜索/序列化放 P1；kitty keyboard/win32-input-mode 默认关 | B. v1 全量 vttest/esctest + 图片/bidi：周期显著上升；C. 只做基本 VT：会破坏 vim/tmux 等日用场景 |
| Q6 | 性能与测试 | A. 用 07 的门槛（10MB/s@60fps、输入 p95 ≤16ms、内存 ≤300MB）+ 09 基线（headless 50.8MB/s、UI 33.4MB/s、app 142MB）；criterion + golden + PTY 集成 + gpui-fps 帧计时 | B. 只做手工验收：无法回归；C. 全量 conformance suite：v1 过重 |
| Q7 | 输入/IME/鼠标 | A. 用 wezterm 的 key_down/key_up + mouse_event + send_paste + focus_changed + bracketed paste；GPUI key_char 喂 IME/文本；kitty/win32 默认关；鼠标 1000/1002/1006 + SGR，Shift 强制本地选区 | B. 自己编码字节：重复且容易错；C. v1 就开 kitty keyboard/win32 input：与 xterm.js 6 默认不一致 |

## 1. TerminalCore façade

    trait TerminalCore {
        fn feed(&mut self, bytes: &[u8]);
        fn resize(&mut self, cols: u16, rows: u16, dpi: u32);
        fn key_down(&mut self, key: KeyInput);
        fn key_up(&mut self, key: KeyInput);
        fn mouse(&mut self, mouse: MouseInput);
        fn paste(&mut self, text: &str);
        fn focus_changed(&mut self, focused: bool);
        fn current_seqno(&self) -> SequenceNo;
        fn screen(&self) -> ScreenView;           // 可见行、光标、模式、标题/cwd/progress
        fn damage_since(&self, seqno: SequenceNo) -> Damage; // 脏行/全量
        fn selection_text(&self, range: CellRange) -> String;
        fn search(&self, query: &str, opts: SearchOptions) -> Vec<SearchMatch>;
    }

- 实现包 wezterm_term::Terminal；TerminalConfiguration：Unicode 11、scrollback 25k、kitty graphics 开、kitty keyboard 关、默认色板。
- 不外泄 wezterm 类型；UI 只看到 ScreenView/Damage/CellRange/SearchMatch。
- title/cwd/progress 直接取 TerminalState 的 get_title/get_current_dir/get_progress；剪贴板用 Clipboard trait 实现，OSC 52 由宿主策略开关。
- 选区、滚动 viewport、搜索 UI 都在 UI 侧；core 只按 CellRange 取文本、按 query 搜屏幕。

## 2. 解析线程与数据流（v1）

    PTY reader thread
      └─ mpsc<Vec<u8>> ── UI tick（8ms）
            ├─ 按字节预算 drain（目标 256KB/tick；上限 1MB/tick）
            ├─ TerminalCore.feed(bytes)
            ├─ damage_since(last_seqno)
            ├─ 更新 TerminalElement 行缓存
            └─ cx.notify()

- 09 数据：1MB/tick 时 UI 吞吐 33.4MB/s、约 30fps；10MB/s 目标用 256KB/tick 应能保住 60fps，11 实现后实测校准。
- 输入顺序：key/mouse/paste 也走同一 UI tick 队列（或直接调用 core，但必须先 drain 完输出再处理输入，避免屏幕状态错位）。
- 如果 10MB/s@60fps 不达标：把 core 移到 worker，UI 发命令、收 Damage/ScreenSnapshot；façade 语义不变。

## 3. 屏幕模型与脏行

- 可见 viewport：screen.phys_row(0) 到 +physical_rows；scrollback 偏移由 UI 的 viewport 决定。
- 初始/全量：resize、alt-screen 切换、scrollback 位移、字体度量变化时整屏重绘。
- 增量：last_seqno = core.current_seqno()；screen.get_changed_stable_rows(visible_stable_range, last_seqno)；把 stable row 映射回 phys row，只重建这些行的 ShapedLine。
- 行缓存：HashMap<StableRowIndex, CachedRow>，CachedRow 存 seqno、ShapedLine、背景/选区 quads 元数据；行内容/属性变化时失效。
- 光标/选区/搜索高亮不进行缓存，画在行之上。

## 4. 渲染管线

- 字体度量：用 shape_line("M") / 实际字体探测 cell width；line height 由字体 metrics + 用户行距配置；CJK 双宽由 core 的 cell width 决定，渲染按 cell 列定位。
- 每行：按颜色/属性切 TextRun（fg、bold、italic、underline、strikethrough），shape_line 一次；paint 在 row origin。
- 背景：按 run/单元格画 paint_quad；默认背景整行一块；选区/搜索高亮叠加。
- 光标：block/underline/bar，闪烁由 UI timer；焦点丢失时空心。
- 图片：v1 P1；core 可解析，渲染先用占位/跳过，v1.x 接 paint_image + 纹理缓存。
- 连字：v1 默认关（与 Tabby 一致）；开启时按行 shape_line 的 ligature 能力，验证列对齐。
- bidi：v1 不做显示重排；core 可关 bidi；P1 再评估。
- 滚动条/搜索框/URL hover 由 TerminalPane 的 GPUI 元素叠加。

## 5. 输入、鼠标、IME

- 键盘：GPUI KeyDownEvent.keystroke → wezterm KeyCode/KeyModifiers；key_char 优先文本（IME/组合输入）；key_down/key_up 都发；altIsMeta、application cursor keys、backspace 四档由 core/config 处理。
- 粘贴：send_paste；bracketed paste 由 core 的 bracketed_paste_enabled 决定；多行确认/换行归一由 UI 做（07 的 18 条）。
- 鼠标：像素 → cell 坐标；mouse_event（按钮/滚轮/修饰键）；core 决定是否上报（is_mouse_grabbed/mouse_tracking_enabled）；Shift 强制本地选区；滚轮在 alt-screen 走 core 的 alternate scroll。
- 焦点：focus_changed；focus tracking 1004 由 core 处理。
- IME：GPUI key_char/平台 IME；候选窗位置 = 光标 cell 的屏幕坐标；预编辑文本不进 core，上屏后作为文本输入；三平台手工矩阵。
- kitty keyboard/win32-input-mode 默认关，与 xterm.js 6 契约一致；需要时按 profile 开。

## 6. xterm.js 6 兼容与测试

03 的 14 项契约作为 P0：
- kitty keyboard 默认 false；win32-input-mode 默认 false；配色查询默认 true；光标闪烁 DECSET 12 默认忽略；ConPTY 补偿按 buildNumber；bracketed paste；OSC 52 策略；OSC 1337 CurrentDir；OSC 8 hyperlink；宽字符 Unicode 11；字体 fallback/emoji。
- P1：search/serialize、图片、minimumContrastRatio/drawBoldTextInBrightColors、bidi。

测试层：
1. Golden 快照：输入字节序列 → ScreenView（行文本/属性/光标/模式）快照；用 xterm.js 6 生成参考（Node headless harness）或手工基准；CI 比对。
2. vttest/esctest 子集：至少覆盖 C0/C1、CSI、DECSET、alt-screen、resize、OSC。
3. PTY 集成：zsh/pwsh/WSL 冒烟；resize/SIGWINCH；ConPTY（Windows CI）。
4. 输入/鼠标单测：key_down 编码、mouse_event 编码、bracketed paste、focus。
5. 性能：criterion feed/reshape；gpui-fps 或自定义 paint 计时，10MB/s@60fps、p95 ≤16ms。
6. Fuzz（可选）：随机字节序列不 panic；OSC 未闭合/超长序列。

## 7. 性能预算

- 07 门槛：冷启 ≤1.5s、热启 ≤500ms、按键 p95 ≤16ms、输出 ≥10MB/s@60fps、8 标签 ≤300MB、搜索 25k 行 <100ms。
- 09 基线：headless 50.8MB/s；UI 33.4MB/s@30fps（1MB/tick）；36 行 CJK/emoji 60fps；app RSS 142MB。
- 11 验收：256KB/tick 下 10MB/s 输出保持 60fps；按键 p95 用 gpui-fps/自定义 paint 测；行缓存命中率/reshape 次数进 benchmark。

## 8. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| wezterm-term 0.1.0 API 不稳定 | 升级/维护成本 | pin commit + façade 隔离；只在上游挡路时 fork |
| GPUI ShapedLine/TextRun 的 paint API 不熟 | 渲染延期 | 11 先做最小 TerminalElement（一行/光标），再逐项加背景/选区 |
| UI 线程解析保不住 60fps | 性能不达标 | 字节预算 + 脏行缓存；不达标切 worker（façade 不变） |
| IME 候选窗/预编辑 | 中文体验 | 原型期手工矩阵；GPUI 平台输入法适配 |
| 图片协议 | 六格/P1 | v1 只保留 core 解析，渲染 v1.x |
| xterm.js 6 参考难自动化 | 兼容回归 | Node xterm.js 6 harness 生成 golden；否则手工基准 + 逐项契约测试 |

## 9. ADR 草稿要点

- ADR-0001：终端内核 wezterm-term git pin + TerminalCore façade；不 fork；alacritty 仅作为砍掉图片/bidi 时的备选。
- ADR-0002：TerminalElement 自绘 + 行级 ShapedLine 缓存 + 脏行 diff；不用 div 行布局。
- ADR-0003：v1 UI 线程解析 + 字节预算；worker 延后，façade 不变。
- ADR-0004：xterm.js 6 兼容契约 14 项 P0 + vttest/esctest 子集 + golden。

## 10. 证据

- 03：.scratch/rust-rewrite/research/03-terminal-engine-pty.md
- 09：.scratch/rust-rewrite/specs/09-prototype-report.md
- 07：.scratch/rust-rewrite/specs/v1-scope.md
- 12：.scratch/rust-rewrite/specs/ui-architecture.md
- wezterm-term：term/src/terminalstate/mod.rs（current_seqno/get_title/get_progress/get_current_dir/mouse_event/send_paste/focus_changed）、screen.rs（get_changed_stable_rows/phys_row/lines_in_phys_range）、terminalstate/mouse.rs。
