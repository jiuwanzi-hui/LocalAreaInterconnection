# LocalAreaInterconnection

[English](#english) | [中文](#中文)

## English

LocalAreaInterconnection is a Windows virtual LAN tool for LAN-only PC games.

It helps players in different places create an encrypted virtual LAN so games that only support local network discovery or direct LAN IP joining can connect over the internet.

### Download

Prebuilt Windows executables will be published on the GitHub Releases page:

[Download from Releases](https://github.com/jiuwanzi-hui/LocalAreaInterconnection/releases)

The repository stores source code only. Release `.exe` files are built separately and uploaded to Releases.

### Features

- Create or join a virtual LAN room.
- Copy room invite codes and virtual IP addresses.
- Show room members and their virtual IPs.
- Use encrypted UDP tunnels between peers.
- Exchange P2P/NAT traversal offers through local or HTTP coordination.
- Forward UDP broadcast traffic used by LAN game discovery.
- Support IPv4 UDP, broadcast, and the core TCP packet path.
- Probe and report Wintun virtual adapter readiness.
- Diagnose adapter, firewall, ping, broadcast, and game traffic issues.
- Export a read-only diagnostic bundle for troubleshooting.
- Explain the selected connection path: direct P2P, relay fallback, or no usable path.
- Switch the desktop UI language between English and Chinese inside the app.

### Current Status

The project is still under active MVP development.

Implemented code already includes the Windows desktop test shell, Rust native CLI, room and invite models, diagnostics, encrypted tunnel envelope, NAT/P2P bootstrap, local JSON coordination store, lightweight HTTP coordination service, UDP forwarding, raw IPv4 UDP/TCP packet handling, connection path assessment, relay fallback planning, and Wintun runtime probes.

Recent native builds also include a small STUN-like UDP observer for endpoint discovery and diagnostic export support for `room-runtime-run` snapshot files. This lets a troubleshooting bundle include runtime packet I/O evidence such as raw virtual packet counts, Wintun send/receive summaries, packet observation lines, and per-peer runtime path and traffic summaries captured during a run.

The desktop test shell can start and stop a controlled native runtime. Runtime snapshots and packet observation files are written under the user's application data folder, and the diagnostic export button automatically includes the latest runtime snapshot when one exists.

Real cross-network gameplay still needs validation on two Windows machines with:

- Administrator permission.
- `wintun.dll`.
- A created and openable Wintun adapter.
- Real game traffic on different NAT networks.

### Which EXE To Run

For normal use, run:

```text
LocalAreaInterconnection.exe
```

Developer and diagnostic builds may also produce:

```text
LocalAreaInterconnection.Cli.exe
LocalAreaInterconnection.Native.Cli.exe
```

Those CLI executables are mainly for testing, diagnostics, and native networking experiments.

### Build From Source

Requirements:

- Windows.
- Rust toolchain with Cargo.
- .NET SDK or the build tools used by the Windows test shell.

Run Rust tests:

```powershell
cd native
cargo test
```

Useful native diagnostics:

```powershell
.\dist\LocalAreaInterconnection.Native.Cli.exe stun-like-serve --bind 0.0.0.0:39120
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-candidates --stun-server <server-ip>:39120
.\dist\LocalAreaInterconnection.Native.Cli.exe diagnostic-export --out diag.json --runtime-snapshot runtime.json
```

Build the latest Windows test shell:

```powershell
.\scripts\build-windows-test-shell.ps1
```

Or double-click:

```text
build-latest-exe.bat
```

Build and launch:

```powershell
.\scripts\run-windows-test-shell.ps1
```

Or double-click:

```text
build-and-run-exe.bat
```

### Development Notes

- `native/` contains the Rust native core and CLI.
- `windows-cli/` contains the current Windows desktop test shell source.
- `scripts/` contains local build helpers.
- Compiled outputs under `dist/` and `native/target/` are not committed.
- Planning and progress documents are local development references and are not required for release downloads.

### License

This project currently uses the license file included in the repository.

## 中文

LocalAreaInterconnection 是一个面向仅支持局域网联机的 PC 游戏的 Windows 虚拟局域网工具。

它帮助不同地点的玩家创建加密虚拟 LAN，让只支持局域网发现或直接输入 LAN IP 加入的游戏可以通过互联网连接。

### 下载

预编译的 Windows 可执行文件会发布到 GitHub Releases 页面：

[从 Releases 下载](https://github.com/jiuwanzi-hui/LocalAreaInterconnection/releases)

仓库只存放源代码。Release 中的 `.exe` 文件会单独构建并上传。

### 功能

- 创建或加入虚拟 LAN 房间。
- 复制房间邀请码和虚拟 IP。
- 显示房间成员和各自的虚拟 IP。
- 在玩家之间使用加密 UDP 隧道。
- 通过本地或 HTTP 协调交换 P2P/NAT 穿透信息。
- 转发局域网游戏发现常用的 UDP 广播流量。
- 支持 IPv4 UDP、广播和核心 TCP 包路径。
- 探测并报告 Wintun 虚拟网卡就绪状态。
- 诊断网卡、防火墙、Ping、广播和游戏流量问题。
- 导出只读诊断包用于排障。
- 解释当前连接路径：P2P 直连、中继兜底，或暂无可用路径。
- 在应用内切换桌面 UI 的中英文。

### 当前状态

项目仍处在 MVP 持续开发阶段。

当前已实现 Windows 桌面测试壳、Rust 原生 CLI、房间与邀请模型、诊断、加密隧道封装、NAT/P2P bootstrap、本地 JSON coordination store、轻量 HTTP 协调服务、UDP 转发、原始 IPv4 UDP/TCP 包处理、连接路径评估、中继兜底计划和 Wintun 运行时探测。

最近的原生构建还加入了一个轻量 STUN-like UDP 观测器，用于端点发现，并支持把 `room-runtime-run` 的 snapshot 文件导入诊断导出。这样故障包里就能包含运行时的 packet I/O 证据，例如原始虚拟包计数、Wintun 收发摘要、运行期间记录的包观测行，以及每个 peer 的运行时路径和流量摘要。

桌面测试壳可以启动和停止一个受控的 native runtime。runtime snapshot 和包观测文件会写到用户的应用数据目录，诊断导出按钮在存在最近 snapshot 时会自动合并进去。

真实跨网络联机仍需要在两台 Windows 机器上验证，并满足以下条件：

- 管理员权限。
- `wintun.dll`。
- 已创建且可打开的 Wintun 网卡。
- 不同 NAT 网络下的真实游戏流量。

### 运行哪个 EXE

正常使用运行：

```text
LocalAreaInterconnection.exe
```

开发和诊断构建还可能生成：

```text
LocalAreaInterconnection.Cli.exe
LocalAreaInterconnection.Native.Cli.exe
```

这些 CLI 程序主要用于测试、诊断和原生网络实验。

### 从源码构建

要求：

- Windows。
- 带 Cargo 的 Rust 工具链。
- .NET SDK，或桌面测试壳所用的构建工具。

运行 Rust 测试：

```powershell
cd native
cargo test
```

常用原生诊断：

```powershell
.\dist\LocalAreaInterconnection.Native.Cli.exe stun-like-serve --bind 0.0.0.0:39120
.\dist\LocalAreaInterconnection.Native.Cli.exe nat-candidates --stun-server <server-ip>:39120
.\dist\LocalAreaInterconnection.Native.Cli.exe diagnostic-export --out diag.json --runtime-snapshot runtime.json
```

构建最新 Windows 测试壳：

```powershell
.\scripts\build-windows-test-shell.ps1
```

或者双击：

```text
build-latest-exe.bat
```

构建并启动：

```powershell
.\scripts\run-windows-test-shell.ps1
```

或者双击：

```text
build-and-run-exe.bat
```

### 开发说明

- `native/` 是 Rust 原生核心和 CLI。
- `windows-cli/` 是当前 Windows 桌面测试壳源码。
- `scripts/` 放本地构建辅助脚本。
- `dist/` 和 `native/target/` 下的编译产物不提交。
- 设计和进度文档只作为本地开发参考，不属于发布内容。

### 许可证

本项目当前使用仓库中附带的 license 文件。
