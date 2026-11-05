<!-- DSH-FORMAT: md-v1 -->
<!-- DSH-PARENT: 00 -->
<!-- DSH-LABELS: wayfinder:research -->
<!-- DSH-IDEMPOTENCY-KEY: deck-plan-rabby-rust-rewrite-v1-child-T04 -->
<!-- DSH-CREATED: 2026-11-05T01:10:03.236Z -->
<!-- DSH-ASSIGNEES: lead -->
# 05: Rust 网络与会话栈选型

Type: research

Status: resolved
## Question

目标：为 SSH/SFTP/端口转发/串口/Telnet/Zmodem 找到 Rust 侧可落地实现，标出与 Tabby 行为的差距。

要回答：
- SSH：russh（纯 Rust）与 ssh2/libssh2 的取舍；认证（password / publickey / keyboard-interactive / agent，含 Pageant 与 Windows OpenSSH agent）、jump host、agent forwarding、port forwarding（local/remote/dynamic）、X11、known_hosts、算法兼容；
- SFTP 客户端能力与并发/断点/权限；
- 串口：serialport crate 的平台覆盖、热插拔、hex 模式、readline；
- Telnet 协议选项与实现；
- Zmodem：Rust 实现/移植可行性（Tabby 用 zmodem.js）；
- Tabby 当前实现（russh npm 0.1.38、node-pty、serialport）能提供的行为基线；
- 每个能力的风险等级与替代方案。

产出：`.scratch/rust-rewrite/research/05-rust-ssh-serial-stack.md`。

## Answer

SSH/SFTP 选 russh 0.65.x + russh-sftp 3.0.x；串口 serialport 4.10.1 + 自建热插拔；Telnet 用 telnet 0.2.5 或自写 IAC codec（v1.x）；Zmodem 用 zmodem2 + 自写 Sentry（v1.x）。
- Tabby 的 russh npm 0.1.38 就是 russh crate 0.63.3 的 napi 绑定（npm 包名 russh-0.1.38+0.63.3.tgz），行为基线同源，Rust 重写风险最低；russh 客户端覆盖 agent（Unix socket / Windows OpenSSH 命名管道 / Pageant）、keyboard-interactive、jump host 材料（direct-tcpip）、local/remote/dynamic 转发、X11、known_hosts（含 |1| 哈希条目）、OpenSSH 证书、ML-KEM 混合 KEX。
- ssh2/libssh2 1.11.1 不选：无 X11 API、Windows 无 Pageant、无 PQ KEX、阻塞式需 spawn_blocking；仅作备选/兜底。
- 必须自建的四块：ProxyJump（russh-config 解析但不实现，照抄 Tabby 的 channel-as-transport）、SOCKS5 服务端（dynamic 转发）、SSH 连接复用池（对齐 Tabby sshMultiplexer）、串口热插拔（serialport 无 hotplug API，macOS 还会重复列 /dev/cu.* 与 /dev/tty.*）。
- hex/readline 属会话中间件而非串口库（Tabby 已有 local-echo/readline/readline-hex/hex 四档，需移植）；Zmodem 生态小（zmodem2 无 Sentry 检测层、lrzsz2 是 GPL），风险中高，放 v1.x feature flag。
研究笔记：.scratch/rust-rewrite/research/05-rust-ssh-serial-stack.md

## 进度
