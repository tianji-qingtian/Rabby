# Rabby 平台、分发与品牌（08 决议）

> 状态：08 票「平台、分发与品牌」的决议，2026-11-05。用户确认 Q1–Q7 全部按推荐 A。
> 输入：07 v1 基线（specs/v1-scope.md）、research 10 的打包/签名约束、02/03/05 的平台与内核结论；品牌与签名成本的一手核查。
> 一句话：macOS 先行、Windows 紧随、Linux 第三；GitHub Releases 直发；开发期不签名，macOS 公开 beta 前补 Apple 公证，Windows 证书 v1.x；Rabby 只作内部代号，公开名前做商标检索；MIT/Apache-2.0 双许可。

## 0. 决议摘要

| # | 决策 | 结论 |
|---|---|---|
| Q1 | 首发顺序 | macOS → Windows → Linux；v1.0 公开 beta 前三个平台都过 P0 |
| Q2 | 安装包与托管 | GitHub Releases；macOS dmg、Windows msi + 便携 zip、Linux AppImage + deb；包管理器 v1.x |
| Q3 | 签名/公证 | 开发期未签名；macOS 公开 beta 前买 Apple Developer Program（99 美元/年）并公证；Windows 证书 v1.x |
| Q4 | 品牌 | Rabby 仅内部代号/仓库名；公开名前做商标检索；不与 Tabby 商标混用 |
| Q5 | 许可证 | MIT/Apache-2.0 双许可 + NOTICE；保留 Tabby MIT 归属与 GPUI Kit/NSH Apache-2.0 归属 |
| Q6 | 发布节奏/更新 | nightly + stable；v1 手动下载更新；v1.x 自动更新 |
| Q7 | Web/移动 | v1/v1.x 不做；Web 在 v1.1 后评估，移动在 NSH 1.0 后评估 |

## 1. 平台与首发顺序

- 顺序：**macOS 先行 → Windows → Linux**。macOS 是开发机，dogfood 最快；Windows 的 ConPTY/shell 发现/环境重建最难，留更多 soak 时间；Linux 打包最简单但用户面较小。
- v1.0 公开 beta 的闸门：三个平台都通过 07 的 P0 验收，不是“macOS 能用就发”。
- 最低系统：macOS 14+；Windows 10 1809+（ConPTY 17692+，reflow 21376+ 分支）；Linux X11 + Wayland（Wayland 全局热键 v1 不做）。
- 架构：macOS universal2（arm64 + x86_64）；Windows x64（arm64 放 v1.x）；Linux x86_64（arm64 放 v1.x）。

## 2. 构建产物与托管

| 平台 | v1 产物 | 工具/说明 | v1.x |
|---|---|---|---|
| macOS | dmg（内含 .app） | universal2；hdiutil/create-dmg；Hardened Runtime + entitlements + 公证 + staple | pkg 安装器、Mac App Store 评估 |
| Windows | msi（WiX）+ 便携 zip | x64；MSI 负责开始菜单/卸载；zip 便携模式 | arm64、Microsoft Store 评估 |
| Linux | AppImage + deb | x86_64；AppImage 自带依赖；deb 声明 libsecret/libnotify/libxkbcommon 等 | rpm、Flathub、Snap、AUR |

- 托管：**GitHub Releases**（remote 已是 github.com/tianji-qingtian/Rabby）。每个 release 附：三平台产物、SHA256SUMS、release notes、已知问题、P0/P1 兼容矩阵。
- 保留最近 N-1 个 release，便于手动回滚。
- 包管理器放 v1.x：Homebrew cask、winget、AUR、Flathub；v1 不接 Mac App Store / Microsoft Store。
- 构建：GitHub Actions 三平台矩阵；macOS/Windows 用原生 runner；Linux 用 Ubuntu 22.04 基线；产物签名/校验步骤进 CI。

## 3. 签名与公证

- **开发期**：未签名/adhoc 构建，只给内部与早期测试；不对外承诺。
- **macOS 公开 beta 前**：加入 Apple Developer Program（99 美元/年，公证不另收费）；用 Developer ID Application 签名；打开 Hardened Runtime；按 10 §1.3 准备 entitlements 与 NS*UsageDescription（PTY 子进程会触发 TCC；缺描述串会被系统直接拒绝）；notarytool 提交、staple、spctl 验证。
- **Windows**：v1 不签名，SmartScreen 警告在下载页与文档中说明；v1.x 再买 OV 证书（约 150–300 美元/年）或 Azure Artifact Signing（约 9.99 美元/月，地区/主体受限）；signtool + RFC3161 时间戳；安装器写 AUMID，通知才能归组。
- **Linux**：无强制签名；发布 SHA256；GPG/minisign 可选 v1.x。
- CI 密钥：APPLE_ID / APPLE_TEAM_ID / APPLE_APP_SPECIFIC_PASSWORD 或 API key；Windows 证书进 secret store；绝不入库。
- 验证闸：codesign --verify --deep --strict、spctl -a -vv、stapler validate；signtool verify /pa；SHA256 校验。

## 4. 品牌

- **Rabby 只作内部代号/仓库名**，不作为公开产品名直接发布。
- 风险事实：Rabby Wallet 是 DeBank 的知名 EVM 钱包（App Store 在架），公开用 Rabby 做终端品牌有混淆与商标风险。
- 公开名前必做：目标市场商标检索（至少 CNIPA/USPTO/EUIPO）、域名/社交 handle 可用性、与钱包/金融/终端类现有品牌避让；名字确定后再改 README、安装包名、App 名称、图标与地图标题。
- 与 Tabby 的关系：Rabby 是独立的 Rust 重写，不隶属于 Tabby；产品内不使用 Tabby 的名称/Logo/图标；只在 NOTICE/README 的“受 Tabby 启发/兼容”段落致谢。
- 时间点：公开 beta 前定名；内部开发不受影响。若你已有心仪名字，下一轮直接给，我更新地图标题与仓库命名建议。

## 5. 许可证与归属

- 本项目：**MIT OR Apache-2.0 双许可**；放 LICENSE-MIT、LICENSE-APACHE、NOTICE。
- Tabby：MIT；保留 “Copyright (c) 2017 Tabby Developers” 与 MIT 全文到 NOTICE/THIRD_PARTY。没有复制 Tabby 源码；如后续移植任何片段，逐文件记录来源与许可证。
- GPUI Kit / NSH：Apache-2.0；保留 NOTICE 与文件头（若上游提供 NOTICE）。
- 依赖审计（16 票落地）：wezterm-term（MIT）、alacritty_terminal（Apache-2.0）、portable-pty（MIT）、russh（Apache-2.0）、serialport（MPL-2.0）等；MPL-2.0 作为依赖可用但要披露来源、不改其源码；**Clink 是 GPLv3，不随包分发**，只探测已安装（10 §0）。
- 贡献政策：DCO（Developer Certificate of Origin），不引入 CLA；CONTRIBUTING 里写明。

## 6. 发布节奏、更新与回滚

- 通道：nightly（main 每次合并，未签名开发包）、beta（预发布，macOS 已签名）、stable（tag）。
- 版本：SemVer 0.x；nightly 用 0.x.y-nightly.YYYYMMDD；stable 用 0.x.y。
- v1：**手动更新**——应用内“检查更新”打开 GitHub Releases（与 Tabby 的 Linux/便携行为一致）；下载后覆盖安装。
- v1.x：自动更新；只对已签名产物开启；支持 staged rollout 与回滚到 N-1；更新源与签名公钥进 16 票设计。
- 回滚：保留 N-1 release；配置 schema 只做向前迁移，迁移前自动备份；文档写手动降级步骤。
- 发布 checklist：P0 测试、三平台产物、SHA256、签名验证、release notes、已知问题、兼容矩阵。

## 7. Web / 移动

- v1/v1.x 都不做；地图 Out of scope 不变。
- Web：v1.1 后单独评估。前置：NSH/WASM 成熟（02：NSH 0.4、WASM 仅 showcase）、Tabby Web 等价的服务端网关/安全模型、远程 PTY 托管。
- 移动：NSH 1.0 后评估；终端输入/IME/软键盘是独立设计，不承诺。
- 触发方式：v1.1 后写一份独立 proposal，不放进当前路线图。

## 8. 遥测 / 隐私

- 默认关闭；v1 不接遥测、不接 Sentry；崩溃上报只在用户显式开启时，且不含终端内容/密钥。
- 本地日志由用户控制；发布页写清隐私说明。
- 后续若要遥测：opt-in、可审计、最小化字段。

## 9. 验收 / 任务清单

- [ ] 公开 beta 前拿到 Apple Developer Program，CI 能签名 + 公证 + staple。
- [ ] Windows 证书决策（v1.x）与 SmartScreen 说明（v1）。
- [ ] GitHub Actions 三平台矩阵出包；产物 + SHA256 + release notes。
- [ ] 干净 macOS/Windows/Linux 虚拟机安装、启动、卸载验证。
- [ ] Gatekeeper/spctl/stapler 与 signtool 验证进 CI。
- [ ] LICENSE-MIT / LICENSE-APACHE / NOTICE / THIRD_PARTY 写完；依赖审计。
- [ ] 公开名前商标检索与命名决策。
- [ ] 发布 checklist、回滚文档、隐私说明。
- [ ] 三个平台都过 07 的 P0 验收后才开公开 beta。

## 10. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| Apple 账号/公证延迟 | 公开 beta 被卡 | 提前 4–6 周申请；先内部 adhoc |
| Windows 证书成本/地区限制 | SmartScreen 警告 | v1 文档说明；v1.x 买 OV 或 Azure（区域确认） |
| 品牌重名 | 改名晚成本高 | 公开 beta 前定名；内部代号不变 |
| 只有 GitHub Releases | v1 无自动更新 | 手动更新 + 检查更新入口；v1.x 自动 |
| PTY 子进程触发 TCC/entitlements | macOS 功能被静默拦 | 按 10 §1.3 准备隐私串与 entitlements，原型期验证 |
| Linux glibc/依赖 | 老发行版跑不起来 | Ubuntu 22.04 基线；AppImage 自带依赖；deb 声明依赖 |
| 许可证污染 | 合规风险 | 不 bundle Clink；依赖审计；NOTICE |

## 11. 证据

- 07 基线：.scratch/rust-rewrite/specs/v1-scope.md
- 10 打包/签名约束：.scratch/rust-rewrite/research/10-platform-integration.md §1.3/§1.4
- 02 UI/平台能力：.scratch/rust-rewrite/research/02-gpui-kit-nsh-capability.md
- 03 内核/PTY：.scratch/rust-rewrite/research/03-terminal-engine-pty.md
- 05 SSH/串口：.scratch/rust-rewrite/research/05-rust-ssh-serial-stack.md
- Rabby Wallet（品牌冲突）：https://apps.apple.com/us/app/rabby-wallet-crypto-evm/id6474381673
- Apple 公证费用：https://developer.apple.com/forums/thread/746992
- Windows 签名选项：https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options
