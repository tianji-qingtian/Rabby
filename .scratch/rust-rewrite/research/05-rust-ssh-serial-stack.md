# 05 Rust 网络与会话栈选型

> 范围：SSH / SFTP / 端口转发 / 串口 / Telnet / Zmodem。
> 上游基准：Eugeny/tabby @ `467ce8e`（本地检出 /tmp/tabby-recon，下文路径均相对该根目录）；Rust crate 版本以 2026-11-05 crates.io 为准。
> 结论分「推荐 / 备选 / 不选」，风险分 低 / 中 / 高。

## 一、结论

**SSH 与 SFTP 用 `russh` 0.65.x + `russh-sftp` 3.0.x，不选 `ssh2`/libssh2。**

决定性证据：Tabby 的 `russh` npm 0.1.38 不是 JS 实现，而是 russh crate **0.63.3** 的 napi-rs 原生绑定——npm 元数据里包描述是 “native bindings for the client side of the russh SSH protocol library”，发布包名为 `russh-0.1.38+0.63.3.tgz`（<https://registry.npmjs.org/russh/0.1.38>）。也就是说 Tabby 今天的 SSH 栈本来就是 Rust：认证计划、agent、jump host、X11、转发全部是 `tabby-ssh` 对同一个 crate 的调用。Rabby 换到 russh 0.65 不是「重写 SSH」，而是把 Node 侧那层胶水搬进进程内，行为基线与上游同源，升级路径也同源。

其余：串口 `serialport` 4.10.1 + 自建热插拔；Telnet 用 `telnet` 0.2.5 或自写 codec；Zmodem 用 `zmodem2` 0.7.3 + 自写 Sentry 检测。Telnet 与 Zmodem 都按 map 约定放 v1.x，不影响 v1 关键路径。

## 二、Tabby 基线（Rabby 要对齐的行为）

| 能力 | Tabby 现状 | 证据 |
| --- | --- | --- |
| SSH 传输 | russh npm 0.1.38（= crate 0.63.3）napi 绑定 | `app/package.json:35`、`tabby-ssh/src/session/ssh.ts:21` |
| 算法选择 | 由 binding 暴露 supported 列表，profile 里再过滤一遍 | `tabby-ssh/src/algorithms.ts:5-9`、`ssh.ts:380-383,431-437` |
| 认证 | none → saved/prompt password → publickey（自动遍历多个 key）→ keyboard-interactive → agent，失败后按 `remainingMethods` 降级 | `ssh.ts:640-810`、`session/authMethodSelection.ts:1-46` |
| Agent（Windows） | 先探测 `\\.\pipe\openssh-ssh-agent`，不存在再试 Pageant；可在设置里强制 pageant / named-pipe | `ssh.ts:24,299-333` |
| Agent（Unix） | 配置路径或 `SSH_AUTH_SOCK`，校验必须是 Unix socket，支持 `@` 抽象命名空间 | `ssh.ts:334-363` |
| Agent forwarding | shell 通道 `auth-agent-req@openssh.com`，收到 agent channel 后桥到本地 agent；未开启则拒绝并打日志 | `ssh.ts:574-597,896-898` |
| Jump host | 先在跳板机上开 session，再 `openTCPForwardChannel` 到目标，把 channel 当传输层 | `sshTab.component.ts:81-114`、`ssh.ts:398-400` |
| ProxyCommand / SOCKS / HTTP | 三条独立 transport 构造；ProxyCommand 拒绝 shell 操作符（`&&`、`\|`） | `ssh.ts:387-416` |
| 端口转发 | local / remote / dynamic 三种；dynamic 是内建 SOCKS5 且 NOAUTH；remote 走 `forwardTCPPort`+`tcpChannelOpen$` | `ssh.ts:502-543,813-872`、`session/forwards.ts:16-50` |
| X11 | 请求 X11 转发（随机 16 字节 MIT-MAGIC-COOKIE-1），收到 x11 channel 连本地 `DISPLAY` 并双向直通 | `ssh.ts:545-572,888-895`、`session/x11.ts:41-52` |
| known_hosts | 存在应用配置 JSON（`ssh.knownHosts: []`），key 为 host+port+type，值为 base64(sha256(pubkey))；**不读 `~/.ssh/known_hosts`** | `services/sshKnownHosts.service.ts:20-32`、`ssh.ts:600-622`、`config.ts:12` |
| 连接复用 | `reuseSession`：以 host/port/user/proxyCommand/socksProxy/jump 递归组合成 key 复用 SSH 会话 | `services/sshMultiplexer.service.ts:31-38` |
| SFTP | 上传先写 `<path>.tabby-upload` 再 rename；支持 chmod、mkdir、递归目录上传/下载、删除；**无断点续传** | `session/sftp.ts:112-150`、`components/sftpPanel.component.ts:170-312` |
| 串口 | `serialport` 11.0.1（`@serialport/bindings-cpp`），仅按需 `list()`，**无热插拔**；打开后 `readable` 直读 | `app/package.json:43`、`tabby-serial/src/services/serial.service.ts:14-28`、`api.ts:74-136` |
| 串口参数 | baud（110–1500000 固定档）、dataBits 5–8、stopBits 1/1.5/2、parity、rtscts/xon/xoff/xany、slowSend | `tabby-serial/src/api.ts:13-29,36-42` |
| hex / readline | 不在串口包，在终端中间件：input `local-echo\|readline\|readline-hex`，output `hex` | `tabby-terminal/src/middleware/streamProcessing.ts:10-11`、`components/streamProcessingSettings.component.ts:21-46` |
| Telnet | 手写 IAC：首包检测 IAC 后 DO SGA、WILL TTYPE、WILL NAWS；`0xFF` 双写转义；无认证 | `tabby-telnet/src/session.ts:46-59,119-136` |
| Zmodem | `zmodem.js` 0.1.9 的 Sentry，装饰器**无条件挂载**（无设置开关），检测到 ZRQINIT 弹确认框 | `tabby-terminal/package.json:39`、`features/zmodem.ts:46-90`、`index.ts:29` |
| 密码存储 | `keytar`（系统钥匙串） | `tabby-ssh/src/services/passwordStorage.service.ts:1,23` |

## 三、SSH 栈对比

| 维度 | russh 0.65.1（推荐） | ssh2 0.9.6 / libssh2 1.11.1_DEV（备选） | 系统 `ssh` 子进程（兜底） |
| --- | --- | --- | --- |
| 实现 | 纯 Rust，tokio 异步，Apache-2.0，MSRV 1.89 | C（libssh2）+ FFI，阻塞 API，需 `spawn_blocking` | 外部进程，PTY/管道通信 |
| 认证 | none / password / publickey / keyboard-interactive / OpenSSH 证书 / 外部 Signer | password / publickey(文件/内存) / keyboard-interactive / agent / hostbased | 全部（由 ssh 自己处理） |
| Agent | Unix socket、Windows OpenSSH 命名管道、**Pageant** | `userauth_agent`；Windows 仅 OpenSSH 命名管道，**无 Pageant** | `SSH_AUTH_SOCK` / Pageant 由 ssh 决定 |
| Jump host | 需自建（`channel_open_direct_tcpip` 当传输层，与 Tabby 同法） | 需自建（`channel_direct_tcpip`） | `-J` 内建 |
| local/remote/dynamic 转发 | `channel_open_direct_tcpip` / `tcpip_forward`+`cancel_tcpip_forward`；dynamic 需自写 SOCKS5 | `channel_direct_tcpip` / `channel_forward_listen`；dynamic 需自写 | `-L/-R/-D` 内建 |
| X11 | 客户端 `request_x11` + `channel_open_x11`；服务端也支持 | **无 X11 API**（src 里搜不到 x11） | `-X/-Y` 内建 |
| known_hosts | 内建 `check_known_hosts`/`learn_known_hosts`，支持 `\|1\|` 哈希条目与默认路径 `~/.ssh/known_hosts` | `KnownHosts` 读/写/check/add，需自己管生命周期 | 完全由 ssh 处理 |
| 算法 | chacha20-poly1305、aes-gcm、etm MAC、ed25519/ecdsa/rsa-sha2、strict-kex、**ML-KEM 混合 KEX** | ecdh/curve25519、chacha20、aes-gcm、rsa-sha2、strict-kex；**无 PQ KEX**；无 cert 用户认证 | 跟随系统版本（最全） |
| 维护 | 0.63.3(2026-09) → 0.65.1(2026-11-03)，pre-1.0 迭代快 | 0.9.5(2025-02) → 0.9.6(2026-06)，稳定但慢 | 跟随 OS |
| 构建 | 默认 `aws-lc-rs`（需 cmake/汇编），可切 `ring` 纯 Rust | 需编译 C + OpenSSL；Windows 交叉编译麻烦 | 无 |

证据：russh 客户端 API `russh/src/client/mod.rs:844-980`、`client/session.rs:103-160,203-281,569-586`；agent 客户端 `keys/agent/client.rs:62-105`；known_hosts `keys/known_hosts.rs:15-137`（哈希条目在 111-121，默认路径 50-56）；配置项 `client/mod.rs:2724-2749`；PQ KEX `kex/hybrid_mlkem.rs`（仓库 <https://github.com/warp-tech/russh>，版本 <https://crates.io/crates/russh>）。
ssh2：`session.rs:335,356,488,507,742,789`、`knownhosts.rs:78-256`、`sftp.rs:179-426`；libssh2 版本见其子模块头文件 `LIBSSH2_VERSION "1.11.1_DEV"`（commit `090f23d`）；Windows agent 只有 `agent_win.c:124,142` 的 `\\.\pipe\openssh-ssh-agent`，全文件搜不到 Pageant。

## 四、逐项判断

**认证。** russh 的 `authenticate_none` 会返回 `remaining_methods`，失败后 `AuthFailure` 也带 `proceed_with_methods`（`client/mod.rs:346-720`），正好复刻 Tabby 的 auth plan（`authMethodSelection.ts`）。注意 Tabby 的 publickey 会自动遍历 keychain / `~/.ssh` 下所有可用 key，并区分「服务器断连是因为试了太多 key」（`ssh.ts:643-647`）——这个降级策略要一起搬。

**Agent。** russh 的 `AgentClient` 有 `connect_uds`/`connect_env`/`connect_pageant`/`connect_named_pipe` 四种（`keys/agent/client.rs:62-105`），与 Tabby 的三种 `AgentConnectionSpec` 一一对应，包括「先探测 Windows 管道再退 Pageant」的顺序。这是 ssh2 明确缺失的能力（Windows 上无法用 Pageant）。

**Jump host。** russh 本身不提供；`russh-config` 虽然解析 `ProxyJump`（`russh-config/src/lib.rs:30-46`），但 `stream()` 只处理 ProxyCommand 或直连（`lib.rs:185-197`），ProxyJump 解析了却没用——这是个坑，别指望它。正确做法照抄 Tabby：在跳板会话上 `channel_open_direct_tcpip` 拿到 `Channel`，用 `ChannelStream` 适配成 `AsyncRead+AsyncWrite` 交给 `client::connect_stream`（Tabby 的 `SshTransport.newSshChannel` 就是这个）。

**转发。** local 用 `channel_open_direct_tcpip`；remote 用 `tcpip_forward` + 监听 `ChannelMsg::OpenForwardedTcpIp`；dynamic 需要自己实现 SOCKS5 服务端（Tabby 用 `socksv5` NOAUTH，只支持 CONNECT）。Rust 侧可用 `fast-socks5` 之类，但为对齐行为（拒绝非法目标、日志、错误信息）建议自写 ~200 行：SOCKS5 握手 + CONNECT + 交给 direct-tcpip，域名解析交给远端（`addressToConnectTo` 可为域名）。

**X11。** russh 客户端有 `request_x11` 与 `channel_open_x11`（`client/session.rs:62-90,153-178`），服务端侧也完整。Tabby 的做法是随机 cookie + 收到 x11 channel 后连本地 `DISPLAY`（`x11.ts:41-52`），不做 Xauthority 查询——Rabby 照搬即可，但要在文档里写明 macOS 需 XQuartz、Windows 需 VcXsrv，且 Linux 上若本地 X server 强制 MIT cookie 会失败（Tabby 同缺陷，属于已知行为而非回归）。

**known_hosts。** Tabby 把主机密钥存在自己的配置里（host+port+type → sha256 base64），**不碰** `~/.ssh/known_hosts`。Rabby 应两者都支持：迁移时导入 Tabby 配置里的 digest；新连接默认同时查 `~/.ssh/known_hosts`（russh 内建，支持 `|1|` 哈希条目与 `[host]:port` 格式）。russh 的已知缺口：不识别 `@cert-authority`/`@revoked` 标记（源码里搜不到），遇到这类条目会被当成不认识的主机——需要在打开文件前自己过滤或降级为提示。ssh2 的 `KnownHosts` 有 `CheckResult` 四态（match/mismatch/not found/failure），语义更接近 OpenSSH，但 API 是 C 风格的手动生命周期。

**算法兼容。** 因为同源，russh 0.65 支持的算法集就是 Tabby 今天能用的集合（含 strict-kex，Tabby 的算法列表里直接出现了 `kex-strict-c-v00@openssh.com`，`algorithms.ts:21-22`）。Rabby 应保留同样的算法勾选 UI 与同样的「profile 值 ∩ 运行时支持」过滤逻辑，并把列表降级为「运行时枚举」而不是硬编码，避免升级 crate 后 UI 与实现不一致。

**连接复用。** Tabby 的 `sshMultiplexer` 以 profile 组合键复用会话（`sshMultiplexer.service.ts:31-38`）。Rust 侧对应一个 `SessionPool`（key 含 host/port/user/proxy/jump，引用计数，最后一个 tab 关闭才断），这是渲染层之外必须显式设计的部分，否则「同一主机开 5 个 tab」会建 5 条 TCP。

## 五、SFTP

推荐 `russh-sftp` 3.0.1（<https://crates.io/crates/russh-sftp>）。协议版本 3（`protocol/mod.rs:68`），客户端 `SftpSession` 提供 open/create/read_dir/rename/symlink/hardlink/statvfs/expand_path（`client/session.rs:38-289`），`File` 实现了 `AsyncRead/AsyncWrite/AsyncSeek`（`client/fs/file.rs:282-358`）。并发参数可配：默认单包 256 KiB、读写各 16 个 in-flight、写包 32 KiB、请求超时 10s（`client/mod.rs:32-52`）。

与 Tabby 的差距与改进点：Tabby 上传是「临时文件 + rename」（`sftp.ts:112-131`），没有断点续传，也没有保留远端权限（上传后用单独 chmod）。Rabby 用 `AsyncSeek` 做 offset 续传是低成本增强；同时要把「远端 rename 可能不覆盖」的兼容处理保留（Tabby 先 `unlink` 再 `rename`）。**风险：低。**

## 六、串口

`serialport` 4.10.1（<https://crates.io/crates/serialport>，MPL-2.0）。平台覆盖：Linux x86_64/aarch64（gnu+musl）、macOS x86_64/aarch64、Windows msvc/gnu、FreeBSD/Illumos/NetBSD（部分无枚举）、Android 无枚举（README “Platform Support”）。

三个必须自己补的点：

1. **热插拔**：crate 只提供 `available_ports()`（`src/lib.rs:1065`），没有任何 hotplug/watch API（全仓 grep 无 hotplug）。Tabby 也没有热插拔，只在打开设置时列一次（`serial.service.ts:18-28`）。Rabby 要做到「拔插自动出现/消失」需平台事件：Linux `udev` monitor、macOS IOKit 通知、Windows `WM_DEVICECHANGE`；退而求其次是 1–2s 轮询 diff，实现成本低、体验够用（推荐先轮询，事件作为增强）。
2. **可枚举性依赖**：Linux 默认走 `libudev`（`Cargo.toml:62-63`），关掉后枚举信息变少、可能列出不存在的口；打包时要带 libudev 或显式关闭并接受降级。macOS 会同时列出 `/dev/cu.*` 与 `/dev/tty.*`，出现重复条目，需要去重或只保留 `cu.*`。
3. **异步与 hex/readline**：crate 是阻塞 IO，需专用线程 + channel，或叠 `tokio-serial` 5.5.0（MIT）。hex 与 readline 不是 crate 能力，是会话层：Tabby 在 `streamProcessing.ts:10-11` 定义了 `local-echo/readline/readline-hex` 输入模式与 `hex` 输出模式，并在 `streamProcessingSettings.component.ts:21-46` 暴露为设置。Rabby 应在 session middleware 层实现同样的四档，而不是塞进 serial 驱动。

MPL-2.0 是文件级 copyleft：链接不传染，但修改其源文件需开源。若不接受，可换 `serial2` 0.2.39（BSD-2/Apache，API 更现代）——功能等价，但生态与文档不如 serialport。**风险：中**（热插拔 + macOS 重复端口 + MPL 合规三件事）。

## 七、Telnet

没有「终端级」的成熟 async crate。`telnet` 0.2.5（MIT，2026-07-08）是阻塞式 `std::net` 客户端，但协议层完整：`Event` 枚举区分 Data/Negotiation/Subnegotiation/UnknownIAC（`src/event.rs:7-24`），`from_stream` 可注入自定义流（`src/lib.rs:178`），`negotiate`/`subnegotiate` 齐备（`lib.rs:359-395`），写数据时自动把 `0xFF` 翻倍（`lib.rs:325-357`）。`mini-telnet` 面向「登录 + 执行命令」，不适合原始终端字节流。

建议：v1.x 用 `telnet` crate 放独立阻塞线程（Telnet 带宽低，线程模型够用），自己驱动协商（对齐 Tabby：DO SGA、WILL TTYPE、WILL NAWS、resize 时发 NAWS）。若后续要完全控制字节流（比如接 Zmodem），再自写 ~300 行 IAC codec 替换，接口不变。**风险：低。**

## 八、Zmodem

`zmodem2` 0.7.3（MIT/Apache，2026-10-10，<https://codeberg.org/jarkko/zmodem2>）是纯状态机：`Sender`/`Receiver` + `poll() -> Action`，调用方拥有全部 IO（`sender.rs:323`、`receiver.rs:353`），no_std、无 async 依赖。它没有 zmodem.js 的 `Sentry` 检测层——**检测必须自己写**：在 session→terminal 的字节流上做滑窗，识别 `ZRQINIT`（`**\x18B0`）与 `ZPAD` 序列，命中后弹确认（对齐 Tabby 的「可能恶意」提示，`zmodem.ts:65-87`），再把字节流接管给状态机，结束后把尾巴（提示符）原样吐回终端——Tabby 为这个「尾字节」专门写了 trailingBuffer（`zmodem.ts:19-35`），移植时不要漏。

不要用 `lrzsz2`（GPL-3.0-or-later，传染）；`rzsz` 是 CLI 且几乎无人用。**风险：中偏高**——生态小（周下载量几千），无现成的终端集成范例，且检测逻辑与渲染顺序耦合；缓解：v1.x、feature flag、失败时降级为普通字节流（行为与未启用一致）。

## 九、风险与替代

| 项 | 等级 | 触发条件 | 缓解 / 替代 |
| --- | --- | --- | --- |
| russh pre-1.0 API 漂移 | 中 | 小版本升级改签名（0.63→0.65 已发生） | 锁定精确版本 + vendor；在 `SshSession` trait 后隔离；每季度评估升级 |
| aws-lc-rs 构建链 | 中低 | CI/打包机无 cmake/nasm，或目标平台不支持 | 切 `ring` feature；Windows/macOS 均验证过 |
| ProxyJump 需自建 | 低 | 用户用跳板机 | 照抄 Tabby 的 channel-as-transport；补集成测试 |
| known_hosts 标记不支持 | 中低 | 用户 known_hosts 有 `@cert-authority`/`@revoked` | 打开前过滤，命中时提示「不受支持，需人工确认」 |
| Pageant 逐渐消失 | 低 | 新版 Windows 只有 OpenSSH agent | 默认顺序照 Tabby（管道优先），Pageant 作为兼容保留 |
| X11 本机鉴权失败 | 低 | Linux 上本地 X server 强制 MIT cookie | 与 Tabby 同缺陷；文档说明；后续可选集成 `xauth` |
| 转发端口权限/冲突 | 中低 | remote forward 被拒、local 端口占用 | 逐条报错到 UI（Tabby 已有 `notifications.error`），不要静默 |
| SFTP 大文件/弱网 | 中低 | 数 GB 传输、断线 | 调 `max_concurrent_*`；用 seek 续传；限制内存缓冲 |
| 串口热插拔 | 中 | 设备拔插、USB hub | 轮询 diff 先行；平台事件增强；拔掉时优雅关闭会话 |
| serialport MPL-2.0 | 低 | 合规审查 | 不改其源码即可；或换 serial2 |
| Telnet 阻塞线程 | 低 | 高并发 telnet 会话 | 一连接一线程，或后续自写 async codec |
| Zmodem 集成 | 中高 | 检测误判、尾字节丢失、与渲染竞争 | feature flag；检测只在 session 层做；用 zmodem.js 行为做 golden 对照 |

## 十、落地建议

依赖：`russh = "=0.65.1"`（`default-features = false, features = ["ring","rsa","flate2"]` 视构建链定）、`russh-sftp = "3.0.1"`、`serialport = "4.10.1"`（或 `serial2`）、`telnet = "0.2.5"`（v1.x）、`zmodem2 = "0.7.3"`（v1.x，可选）。

自研模块（约 6 个）：① `Transport` 抽象（TCP / ProxyCommand 子进程 / 跳板 channel / SOCKS / HTTP CONNECT），对齐 `ssh.ts:386-419`；② `AuthPlan`（复用 Tabby 的降级顺序与错误文案）；③ `ForwardManager`（local/remote/dynamic + SOCKS5 server）；④ `KnownHostsStore`（应用 JSON + `~/.ssh/known_hosts` 双读，写入策略与 Tabby 一致）；⑤ 串口 `PortWatcher`；⑥ `ZmodemSentry` + IAC codec（后两者 v1.x）。

验收基线：用同一台 Linux 服务器跑 Tabby 与 Rabby 对照——password/publickey/kbd-interactive/agent 四种认证、带口令的私钥、跳板机、local/remote/dynamic 三种转发、X11（有 X server 时）、known_hosts 首次确认与变更告警；SFTP 上传/下载/递归/改名/chmod；串口用 USB 转串口设备做拔插与 hex 回显。以上任一不通过都算回归，因为上游行为已由 Tabby 固定。

未决：① 是否接受 pre-1.0 依赖（建议接受，替代方案代价更大）；② Zmodem 是否进 v1（建议不进）；③ Windows 上是否保留 Pageant 测试矩阵（建议保留但降优先级）。

---

### 参考

- Tabby 源码（本地 /tmp/tabby-recon，@467ce8e）：`app/package.json`、`tabby-ssh/src/**`、`tabby-serial/src/**`、`tabby-telnet/src/session.ts`、`tabby-terminal/src/features/zmodem.ts`、`tabby-terminal/src/middleware/streamProcessing.ts`
- npm russh 0.1.38：<https://registry.npmjs.org/russh/0.1.38>
- russh crate 0.65.1：<https://crates.io/crates/russh>，源码 <https://github.com/warp-tech/russh/tree/v0.65.1>
- russh-sftp 3.0.1：<https://crates.io/crates/russh-sftp>
- ssh2 0.9.6：<https://crates.io/crates/ssh2>；libssh2 1.11.1_DEV 子模块 `090f23d`
- serialport 4.10.1：<https://crates.io/crates/serialport>，<https://github.com/serialport/serialport-rs>
- telnet 0.2.5：<https://crates.io/crates/telnet>
- zmodem2 0.7.3：<https://crates.io/crates/zmodem2>，<https://codeberg.org/jarkko/zmodem2>
