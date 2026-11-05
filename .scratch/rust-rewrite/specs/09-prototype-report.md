# 09 终端垂直切片原型：测量报告（v0.2）

> 状态：09 票 prototype 产物，2026-11-05。原型是 throwaway，不进入产品架构；结论写回 11/12。
> 问题：GPUI Kit 0.9 + wezterm-term + portable-pty 能不能跑？吞吐、内存、PTY resize、CJK、GPUI 帧时间是什么量级？
> 环境：macOS 27.2 arm64；Rust 1.101.0-nightly (2026-11-04)；GPUI Kit 0.9.0（gpui-pre 0.3.11）；wezterm-term git e68a8ec (2026-10-26)；portable-pty 0.9.0。

## 1. 原型结构

- prototype/terminal-spike：GPUI Kit 窗口 + wezterm-term + portable-pty；36 行文本网格、键盘输入、窗口 resize、PTY 输出 feed；SPIKE_CMD 注入自动命令，SPIKE_ANIM 注入 36 行 CJK/emoji 动画。
- prototype/terminal-headless：纯 headless（无 GPUI），跑 VT 检查、CJK 宽度、PTY resize/SIGWINCH、32MB 吞吐与 RSS。
- prototype/vendor/pathfinder_simd：为 Rust nightly 1.101 / 1.94 打的本地补丁（见 §2）。
- 截图：09-hello.png（PTY 输出 HELLO）、09-highload.png（32MB 输出）、09-anim.png（36 行 CJK/emoji 动画），都在 .scratch/rust-rewrite/prototype/。

## 2. 构建过程与坑

1. 环境默认 rustc 1.94.0-nightly (2025-12-22) 太旧：gpui-pre 0.3.11 使用 std::hint::cold_path 的稳定化版本，1.94 直接 E0658。安装 nightly 1.101.0 (2026-11-04) 后通过。
2. pathfinder_simd 0.5.6 在 aarch64 上使用被移除的 std::intrinsics::simd::simd_minimum_number_nsz / simd_maximum_number_nsz；本地 vendor 后改用 aarch64::vminq_f32/vmaxq_f32/vminq_s32/vmaxq_s32，GPUI 才能编译。这是 GPUI Kit 0.9 当前的供应链风险，11/16 要决定 pin/vendor 策略。
3. Cargo registry 默认写 HOME 被沙箱拒绝：原型用工作区内的 .cargo-home / .rustup-home（不提交）。产品 CI 不应依赖这一点。
4. wezterm-term 没有 crates.io 发布；git 依赖会拉整个 wezterm 仓库与若干子模块（约 800MB），首次构建偏慢。11 要决定 git pin + vendor，还是第三方转发。

## 3. 实验 1：吞吐、scrollback、内存（headless release）

命令：cd prototype/terminal-headless && ./target/release/terminal-headless all。

| 指标 | 结果 | 说明 |
|---|---|---|
| 吞吐 | 33,974,239 bytes / 0.669 s = 50.8 MB/s | 32MB 80 列文本经 zsh + PTY + wezterm-term 解析；release |
| 首字节 | 23.8 ms | 从 spawn 到 reader 第一块 |
| scrollback | 25,024 行 | 配置 25,000，含 visible 行 |
| RSS 峰值 | 36.5 MB | 含进程、wezterm-term、25k 行；macOS ru_maxrss |
| debug 对照 | 2.6 MB/s | debug 构建，不能作为性能结论 |

结论：wezterm-term 解析吞吐在 release 下达 50 MB/s 量级，覆盖日常输出；25k scrollback 的模型内存约 36MB 量级（不含 GPUI/GPU），对 v1 的 300MB 预算友好。

## 4. 实验 2：PTY resize / SIGWINCH（macOS zsh）

headless resize：zsh 里 trap 'echo WINCH; exit 0' WINCH，portable-pty 的 master.resize(100x30) + wezterm-term resize 后：
- SIGWINCH=true；term=100x30。
- 结论：portable-pty 在 macOS 上能正确把 resize 传到子进程；Windows ConPTY 与老 build reflow 未测（本机无 Windows），留给 11。

## 5. 实验 3：GPUI 渲染

- 窗口能打开；PTY 输出经 wezterm-term 解析后，GPUI 视图渲染出 HELLO（09-hello.png）；32MB 输出也渲染成 36 行文本（09-highload.png）。
- 36 行 CJK/emoji 动画（09-anim.png）：约 60fps；Render 回调 0.01ms avg / 0.02ms max。注意：GPUI 的 Render 回调只构造元素树，text shaping/layout/paint 在之后发生，所以这不是完整帧时间；真帧时间要用 gpui-fps/custom Element paint 再测。
- 高吞吐（release，frontmost，byte budget 1MB/tick）：32MB 在约 1s 内 feed，约 33.4 MB/s；期间约 30 frames（≈30fps）；first_frame 0.6ms；进程 RSS 约 142MB（145,504 KB）。
- 高吞吐（debug）：feed 约 0.6 MB/s，CPU 82%，RSS 160MB；主因是 debug 下 wezterm-term 解析慢。
- 关键坑：第一版每 8ms tick 最多消费 32 个 chunk；PTY 的 chunk 只有约 1KB，于是吞吐被卡在约 3.4MB/s。改成按字节预算（1MB/tick）后升到 33.4MB/s。结论：渲染循环必须按字节/时间预算，不能按 chunk 数。
- 被遮挡/后台时 GPUI 会节流：同样 byte budget 在后台只有 3.4MB/s；frontmost 才 33.4MB/s。性能验收必须在真实前台窗口做。
- 生产结论：终端网格不能像原型一样用 36 个 div 文本行渲染；需要自定义 Element + shaped line/glyph + 脏行 diff（11/12 落地）。
- CJK/IME：headless 宽度检查通过（13 chars / 17 cells，含 emoji 与组合字符）；GPUI 里 CJK 字形正常（09-anim.png）；IME 候选窗/预编辑未自动化验证，需手工矩阵。

## 6. 实验 4：xterm.js 6 / VT 兼容

headless 通过的 5 项：
1. CSI 2J + H 清屏/归位。
2. OSC 0 标题。
3. alt screen enter/leave（DECSET 1049）。
4. 模型 resize 100x30。
5. 宽字符/emoji/组合字符宽度（13 chars → 17 cells）。

未覆盖：vttest/esctest 全量、kitty keyboard 默认关、win32-input-mode、配色查询、ConPTY reflow、bracketed paste、OSC 52/1337/8 的完整行为、search/serialize、图片协议。这些必须在 11 的兼容测试套件里补齐；本原型只证明“能解析常见序列”。

## 7. 对 11/12 的结论

1. wezterm-term 可行，但需要 TerminalCore façade 隔离：输入字节/键鼠事件进，屏幕快照/脏行/告警出。Screen::visible_lines 是 test-only，生产要用 phys_row(0) + lines_in_phys_range 或维护 viewport 缓存。
2. GPUI Kit 0.9 可以承载外壳与文本渲染，但终端 widget 必须自研；原型的 div 行布局只适合验证，不适合生产。
3. PTY feed 必须按字节/时间预算，不能按 chunk 数；否则小块 PTY 输出会把吞吐压到 3.4MB/s。高吞吐时帧率约 30fps（div 行 + 1MB/tick），生产要用脏行 diff 保住 60fps。
4. 构建链要固定：Rust nightly 版本、gpui-pre 快照、pathfinder_simd 补丁、wezterm git pin。11/16 决定是否 vendor。
5. 性能门槛需要 release 的稳定帧时间与 UI 吞吐；本报告给出 headless 50.8MB/s、UI 33.4MB/s、动画 60fps、RSS 142MB 作为 07 §3.3 的校准输入，但真帧时间还要补 gpui-fps/custom paint 测量。
6. IME/CJK 是下一步手工验证项；本机 macOS 中文输入法需要实际敲入，不能只靠字节注入。

## 8. 复现

- 构建：cd prototype/terminal-spike && RUSTUP_HOME=$PWD/../../.rustup-home CARGO_HOME=$PWD/../../.cargo-home rustup run nightly cargo build --release
- headless：cd prototype/terminal-headless && ./target/release/terminal-headless all（需要 PTY 权限）
- GPUI：cd prototype/terminal-spike && SPIKE_CMD='cat /tmp/spike-32mb.txt' ./target/release/terminal-spike
- 动画：SPIKE_ANIM=1 ./target/release/terminal-spike
- 32MB fixture：python3 生成 80 列文本到 /tmp/spike-32mb.txt。

## 9. 未完成 / 下一步

- 真帧时间：用 gpui-fps HUD 或自定义 Element 的 paint 计时，补完整帧时间；当前只有 Render 回调与端到端帧数。
- Windows ConPTY / 老 build reflow（本机无 Windows）。
- IME 手工矩阵（macOS 中文输入法）。
- vttest/esctest 子集（至少 14 项契约）。
- 自定义 Element + 脏行 diff 的最小实现（11 的第一块）。
