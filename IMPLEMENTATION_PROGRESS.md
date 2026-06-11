# LocalAreaInterconnection 实施进度文档

## 使用说明

这个文档用于后续断断续续记录实施进度、决策、问题和下一步计划。

每次推进后，建议只补充三类内容：

- 本轮完成了什么。
- 遇到了什么问题或风险。
- 下一轮从哪里继续。

## 任务总览

| 任务 | 状态 | 备注 |
|---|---|---|
| Windows 桌面测试壳入口 | 已完成 | `dist/LocalAreaInterconnection.exe` 是可双击桌面入口，`LocalAreaInterconnection.Cli.exe` 是 CLI 后端。 |
| CLI 房间创建/解析/加入 | 已完成 | C# 测试壳和 Rust CLI 均已有基础命令。 |
| 邀请码复制、虚拟 IP 复制 | 已完成 | 已接入桌面测试壳。 |
| 房间详情摘要 | 已完成 | 当前展示本地状态和诊断摘要，尚未接入真实成员服务。 |
| 网卡计划/扫描诊断 | 部分完成 | 已有 dry-run 计划、netsh 文本解析和测试壳扫描入口；真实虚拟网卡安装/启停未接入。 |
| 防火墙计划/诊断 | 部分完成 | 已有 dry-run 计划、规则诊断和 netsh 文本解析；真实管理员权限修改未验证。 |
| UDP/TCP/广播测试 | 部分完成 | 测试命令和桌面按钮已可生成 packet observation；还不是真实游戏流量捕获。 |
| 网络观测与下一步动作 | 部分完成 | 已合并 adapter/tunnel/P2P/broadcast/game traffic 观测并输出建议；真实隧道状态未接入。 |
| 诊断导出包 | 部分完成 | C# 测试壳和 Rust core/CLI 均可导出 JSON bundle；真实服务采集仍未接入。 |
| Rust 工具链与 native 测试 | 已完成 | `cargo 1.96.0`、`rustc 1.96.0` 可用；`native/` 下 `cargo test` 通过 20 个核心测试。 |
| Rust 原生代码格式化 | 已完成 | 已执行 `cargo fmt`，并在格式化后复测通过。 |
| 真实虚拟网卡集成 | 未完成 | 需要选择/验证 Wintun/TAP/第三方方案并做管理员权限测试。 |
| 真实隧道服务 | 未完成 | 已有 tunnel service snapshot 到诊断观测的转换边界；P2P、加密、心跳、重连、路径统计服务仍未实现。 |
| 真实广播/游戏包捕获与转发 | 未完成 | 已有 packet capture summary 到诊断观测的转换边界；真实虚拟网卡抓包和转发仍未实现。 |
| 真实房间成员生命周期 | 部分完成 | Rust core 已有本地房间成员生命周期模型；尚未接入真实协调服务、在线心跳或桌面详情。 |
| Git 仓库初始化 | 已完成 | 已执行 `git init`，补充 `.gitignore`，并创建首次基线提交。 |

## 当前状态

状态：Windows 客户端 Rust 原生核心继续推进；已新增网络观测诊断边界、Windows 网卡 netsh 观测解析、UDP/TCP 包观测文件链路、ping 连通性观测、UDP 广播测试入口和只读诊断导出包；已生成可双击启动的 Windows 桌面测试程序与 CLI 后端；Rust 工具链已可用，`native/` 已通过编译和测试

最后更新：2026-06-11

负责人：待填写

当前仓库状态：

- 已有 `FUNCTIONAL_DESIGN.md`，内容覆盖产品定位、MVP 范围、网络设计、模块划分、里程碑和风险。
- 已有 `IMPLEMENTATION_PROGRESS.md`，本次已根据功能设计文档重写为可执行计划，并同步为实施进度。
- 已新增 `native/` Rust workspace，作为最终 Windows 客户端的高效率原生核心方向。
- 已生成 `native/Cargo.lock`，并完成当前依赖锁定。
- 已删除早期脚本原型文件，避免后续偏离 Windows 桌面端 + Rust 原生核心方向。
- 已在 Rust 原生核心新增 `game_network_plan` 能力，可将游戏模板转换为防火墙端口规则、广播策略、诊断检查项和操作建议。
- 已在 Rust 原生核心新增 `firewall_plan` 能力，可生成 Windows `netsh advfirewall` dry-run 命令和回滚命令，但不会实际修改系统防火墙。
- 已在 Rust 原生核心新增 `firewall_diagnostics` 能力，可根据期望规则和观测规则判断 Windows 防火墙规则是否缺失、禁用或配置不匹配。
- 已在 Rust 原生核心新增 `network_observation` 能力，可将网卡、隧道、P2P、广播包、游戏流量观测合并为诊断报告。
- 已在 Rust 原生核心新增 Windows 网卡 `netsh interface ipv4 show config` 输出解析能力，可生成 `AdapterObservation` 并接入网络观测报告。
- 已在 Rust 原生核心新增 packet observation 文本解析能力，可将 UDP/TCP 观测行接入广播和游戏流量诊断。
- 已在 Rust 原生核心新增 Windows ping 输出解析能力，可将互 ping 结果转换为 `TunnelObservation`，用于延迟、丢包、P2P 状态诊断。
- 已在 Windows 测试程序新增 UDP 广播测试入口，可生成 broadcast packet observation，用于验证广播诊断链路。
- 已在 Windows 测试程序新增 `diagnostic-export` 只读诊断导出入口，可将网卡、防火墙、ping、包观测和综合网络诊断写入 JSON bundle。
- 已修正 Windows 测试程序的防火墙 `netsh` 解析，支持中文 Windows 输出中的 `规则名称`、`协议`、`本地端口`、`已启用` 和 `是/否`。
- 已新增 `windows-cli/` Windows 测试程序源码，并编译出：
  - `dist/LocalAreaInterconnection.exe`：可双击启动的桌面测试壳。
  - `dist/LocalAreaInterconnection.Cli.exe`：桌面壳内部调用和命令行测试用的 CLI 后端。
- 已删除旧的 `dist/LocalAreaInterconnection.Desktop.exe` 产物，避免用户误点命令行程序或混淆入口。
- 桌面测试壳已支持创建/解析/加入房间、复制邀请码、复制虚拟 IP、网卡/防火墙/网络诊断、UDP/TCP/广播测试写入包观测文件、导出诊断包和右侧房间详情摘要。
- Rust 工具链当前可用：`cargo 1.96.0`、`rustc 1.96.0`。
- `native/` 已完成 `cargo fmt` 和 `cargo test`，当前 20 个 `lai-core` 测试通过，`lai-cli` 测试目标可编译。
- 当前目录已初始化为 Git 仓库，并已创建首次基线提交。

## 本轮进展

### 2026-06-11 本次会话：修复 IDE 运行误报与生成最新版 exe 入口

已完成：

- 根据截图确认：IDE 里点击 `Run lai-cli` 报 `exit code: 2`，原因是 Rust CLI 未传子命令运行时 clap 按错误退出；不是代码编译失败。
- 根据后续截图确认：`Build latest Windows exe` / `Build and run Windows exe` 运行配置红叉的原因是 IDE 没有解析到 `powershell.exe` 解释器路径。
- 修改 `native/crates/lai-cli/src/main.rs`：
  - `command` 改为可选。
  - 无参数运行时打印 help 并正常退出 0。
- 更新 `native/crates/lai-cli/tests/cli_smoke.rs`：
  - 新增无参数运行测试，确保以后 `Run lai-cli` 不再因为只打印帮助而显示红色错误。
- 新增 `scripts/build-windows-test-shell.ps1`：
  - 一键重新编译 `dist/LocalAreaInterconnection.exe`。
  - 同时重新编译 `dist/LocalAreaInterconnection.Cli.exe`。
- 新增 `scripts/run-windows-test-shell.ps1`：
  - 先构建最新版 Windows 测试壳。
  - 再启动 `dist/LocalAreaInterconnection.exe`。
- 新增 JetBrains 运行配置：
  - `.run/Build latest Windows exe.run.xml`
  - `.run/Build and run Windows exe.run.xml`
- 修正 JetBrains 运行配置：
  - 将解释器从 `powershell.exe` 改为 `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`。
  - 移除运行配置解释器参数中的 `-File`，让 IDE 自己传入脚本文件路径。
- 新增根目录双击入口：
  - `build-latest-exe.bat`：生成最新版 exe。
  - `build-and-run-exe.bat`：生成并启动最新版桌面 exe。
- 更新 `README.md`，说明如何生成和启动最新版 exe。

测试结果：

- 在 `native/` 下执行 `cargo fmt`：通过。
- 在 `native/` 下执行 `cargo test`：通过。
  - Rust CLI 集成测试：5 个通过。
  - Rust core 单元测试：26 个通过。
- 执行 `cargo run -q -p lai-cli`：正常打印 help，退出码 0。
- 执行 `.\scripts\build-windows-test-shell.ps1`：成功生成：
  - `dist\LocalAreaInterconnection.exe`
  - `dist\LocalAreaInterconnection.Cli.exe`
- 执行 `.\scripts\run-windows-test-shell.ps1`：成功构建并启动 `dist\LocalAreaInterconnection.exe`，进程保持运行。
- 执行 `build-latest-exe.bat`：成功生成两个 exe。
- 执行 `build-and-run-exe.bat`：成功生成并启动 `dist\LocalAreaInterconnection.exe`，进程保持运行。

使用提示：

- 只想生成最新版 exe：在 IDE 右上角运行配置下拉里选 `Build latest Windows exe`，再点绿色运行按钮；或者双击 `build-latest-exe.bat`。
- 想生成并直接打开：选 `Build and run Windows exe`，再点绿色运行按钮；或者双击 `build-and-run-exe.bat`。
- 手动双击运行：打开 `dist\LocalAreaInterconnection.exe`。
- 不建议点 `lai-cli` 当桌面程序用；它只是命令行后端，点它只会显示命令帮助。

### 2026-06-11 本次会话：Rust 诊断导出、房间生命周期、运行时观测边界

已完成：

- 读取 `IMPLEMENTATION_PROGRESS.md`，确认下一步优先级包括：
  - 将 `diagnostic-export` 从 Windows C# 测试壳迁移到 Rust core/native service 边界。
  - 为 Rust CLI 增加集成测试，固定关键 JSON 输出 schema。
  - 将 `network_observation` 接入真实隧道服务状态和真实广播/游戏包捕获摘要。
  - 将房间详情逐步接入真实成员列表、连接路径、延迟和生命周期操作。
  - 初始化 Git 仓库。
- 对照 `FUNCTIONAL_DESIGN.md`，选择继续推进诊断导出、房间成员状态、隧道/抓包观测边界和可持续工程基建。
- 新增 `native/crates/lai-core/src/diagnostic_export.rs`：
  - 定义 Rust 诊断导出 bundle schema。
  - 将环境信息、输入参数、adapter scan、firewall scan、ping、packet observation、network observation 合并到只读 JSON bundle。
  - 支持把 adapter/firewall netsh 文本、ping 输出和 packet observation 统一转为诊断 section。
  - 增加 bundle 单元测试。
- 更新 `native/crates/lai-cli/src/main.rs`：
  - 新增 Rust CLI `diagnostic-export` 子命令。
  - 支持 `--out` 写 bundle 文件。
  - 支持 `--adapter-netsh-output` / `--firewall-netsh-output` / `--ping-output` / `--packet-observations` 输入文件。
  - 支持无文件时只读运行 `netsh` 和 `ping` 采集当前机器文本输出。
  - 支持 `--adapter-scan false` 和 `--firewall-scan false` 这种显式布尔参数，方便与现有 C# 测试壳参数习惯兼容。
- 新增 `native/crates/lai-cli/tests/cli_smoke.rs`：
  - 覆盖 `init` 输出房间和邀请码。
  - 覆盖 `network-observe` 输出 adapter/broadcast/game traffic 诊断摘要。
  - 覆盖 `diagnostic-export` 写入 JSON bundle 文件。
  - 覆盖 `room-summary` 输出房间会话成员摘要。
- 新增 `native/crates/lai-core/src/room_lifecycle.rs`：
  - 定义本地房间会话、成员、角色、在线/离开状态、连接路径、延迟、丢包和关闭房间状态。
  - 支持创建房间 session、添加成员、更新成员连接状态、成员离开、房主关闭房间。
  - 输出 `RoomSessionSummary`，用于后续桌面房间详情面板接入。
- Rust CLI 新增 `room-summary` 子命令，可输出本地房间 session 和 summary，用于验证成员列表/在线状态/下一步动作结构。
- 新增 `native/crates/lai-core/src/runtime_observation.rs`：
  - 定义 `TunnelServiceSnapshot`，用于未来真实 tunnel service 输出运行状态。
  - 定义 `PacketCaptureSummary`，用于未来真实虚拟网卡抓包服务输出广播/游戏流量摘要。
  - 提供转换函数，将 tunnel service snapshot 和 packet capture summary 接入 `NetworkObservationSnapshot`。
- 更新 `README.md` 和 `docs/ARCHITECTURE.md`：
  - 补充 Rust `diagnostic-export`、`room-summary`、房间生命周期模型和运行时观测转换边界。
  - 修正 Rust 工具链不可用的过期说明。
- 更新 `.gitignore`：
  - 增加 `.idea/`。
  - 增加 `native/target/`。
- 执行 `git init`，当前目录已初始化为 Git 仓库。
- 创建首次基线提交：`dbf167e Initial LocalAreaInterconnection baseline`。

测试结果：

- `cargo fmt --check`：通过。
- `cargo test`：通过。
  - Rust CLI 集成测试：4 个通过。
  - Rust core 单元测试：26 个通过。
  - doc-tests：通过。
- CLI 冒烟：
  - `diagnostic-export` 可写入临时 JSON bundle，stdout 返回 `status=ok`，bundle `schema_version=1`。
  - `room-summary --peer Bob --peer Carol` 输出 `member_count=3`、`online_count=3`、`status=Open`。
- 首次基线提交已创建，后续仅剩本进度文件更新需要追加提交。

未完成：

- 真实虚拟网卡安装/启停、真实虚拟网卡抓包和真实广播转发仍未实现。
- 真实 P2P/加密隧道服务、NAT 打洞、心跳、重连和路径统计仍未实现。
- `runtime_observation` 当前是服务接入边界，不是实际 tunnel/capture 服务。
- `room_lifecycle` 当前是本地状态模型，不是协调服务、心跳服务或真实在线成员同步。
- 桌面测试壳仍调用 C# CLI 后端，尚未切换到 Rust CLI/native service。
- Git 仓库已初始化并创建首次基线提交。

阻塞问题：

- 真实虚拟网卡、真实隧道和真实抓包需要确认 Wintun/TAP/第三方方案、管理员权限和至少两台 Windows 测试环境。
- 如果要开始真实防火墙修改、虚拟网卡配置或驱动安装，需要管理员终端和明确授权。
- 如果要切换桌面壳到 Rust CLI/native service，需要决定短期继续 WinForms 壳，还是启动 Tauri/WinUI 壳。

下一步建议：

- 在真实系统能力前，继续把 Rust native core 边界补完整：coordination 房间事件模型、tunnel service trait、packet capture trait、diagnostic-export 稳定 schema 测试。
- 决定虚拟网卡路线：Wintun、TAP、封装现有 VPN，或先用 WireGuard/ZeroTier 路线验证。
- 准备两台 Windows 测试环境后，开始 M1：虚拟 IP 互 ping、UDP 单播、广播捕获/转发。
- 创建首次 Git 提交，固定当前 Rust core/CLI 和文档基线。

### 2026-06-11 本次会话：Rust 验证与任务表

已完成：

- 读取 `IMPLEMENTATION_PROGRESS.md`，确认上一轮主要阻塞是 Rust 工具链不可用，且真实虚拟网卡、真实隧道、真实包捕获、真实房间成员服务仍未接入。
- 对照 `FUNCTIONAL_DESIGN.md`，选择先验证并修复 `native/` Rust 原生核心，而不是重复推进已完成的 WinForms 测试壳功能。
- 在文档头部新增“任务总览”表，汇总已完成、部分完成、未完成任务。
- 确认当前环境中 `cargo` 和 `rustc` 已可直接使用：
  - `cargo 1.96.0 (30a34c682 2026-05-25)`
  - `rustc 1.96.0 (ac68faa20 2026-05-25)`
- 首次执行 `native/` 下 `cargo test`，定位到两个 Rust 编译问题：
  - `room.rs` 中 `Ipv4SubnetSerde(String)` 错误派生 `Copy`。
  - `lai-cli/src/main.rs` 中 `Option::map(...).transpose()?` 的 `Ok(...)` 缺少错误类型上下文。
- 修复 Rust 编译问题：
  - 去掉 `Ipv4SubnetSerde` 的 `Copy` 派生。
  - 为 CLI adapter observation 闭包中的 `Ok` 显式标注 `Box<dyn std::error::Error>` 错误类型。
- 执行 `cargo fmt`，统一格式化 `native/` Rust workspace。
- 运行 Rust CLI 冒烟验证：
  - `cargo run -p lai-cli -- init --room-name "Friday LAN" --host Alice` 可生成房间和邀请码。
  - `cargo run -p lai-cli -- network-observe ...` 可输出网络诊断 JSON，并识别 `broadcast=seen`、`game_traffic=seen`、`tunnel=ok`、`p2p=ok`。
- 记录一个验证细节：Rust CLI 的 `--packets` 参数格式是 `protocol:source_ip:destination_ip:port:broadcast|unicast:direction:bytes`。

测试结果：

- `cargo fmt`：已执行。
- `cargo test`：通过。
  - `lai-core`：20 个测试通过。
  - `lai-cli`：测试目标可编译，当前没有单元测试。
  - doc-tests：通过。
- CLI 冒烟：
  - `init` 命令通过。
  - `network-observe` 命令通过；由于命令只传了 `expected_ip`、未传 `assigned_ip`，输出中的 `virtual_adapter=ip-mismatch` 符合当前模型预期。

未完成：

- 本轮没有继续新增真实虚拟网卡、真实隧道、真实包捕获或真实房间服务代码。
- `diagnostic-export` 仍在 C# 测试壳中，尚未迁回 Rust core/service。
- Rust CLI 尚缺少围绕命令输出 schema 的自动化集成测试。
- `native/target/` 是本地构建产物，不应作为源码进度依赖。

阻塞问题：

- Rust 工具链阻塞已解除。
- Git 仓库当时尚未初始化；该问题已在 2026-06-11 处理，目前尚未创建首次提交。
- 真实系统网络能力仍需要管理员权限、虚拟网卡方案和两台 Windows 测试环境。

下一步建议：

- 优先把 C# 测试壳中的 `diagnostic-export` schema 迁入 Rust `lai-core` 或 native service 边界，并补测试。
- 为 Rust CLI 增加集成测试，固定 `init`、`game-plan`、`firewall-plan`、`network-observe` 的 JSON 输出形状。
- 开始真实采集层设计：隧道服务状态、虚拟网卡包捕获摘要、广播转发摘要。
- 初始化 Git 仓库，至少在继续大范围实现前建立变更追踪。

### 2026-06-11 本次会话

已完成：

- 读取 `IMPLEMENTATION_PROGRESS.md`，确认当前基线仍是 Rust 工具链缺失、真实虚拟网卡/隧道/包捕获未接入，Windows C# 测试壳作为当前可运行验证入口。
- 对照 `FUNCTIONAL_DESIGN.md`，选择继续改进桌面测试壳的产品流程，而不是重复新增底层测试命令：
  - 功能设计要求“小白可用：创建房间、复制邀请码、加入房间、启动游戏”。
  - 功能设计要求“游戏辅助：一键复制自己的虚拟 IP”。
  - 功能设计要求“诊断模块：日志打包”。
- 修正 Windows 发布产物入口：
  - `dist/LocalAreaInterconnection.exe` 现在是可双击启动的桌面程序。
  - `dist/LocalAreaInterconnection.Cli.exe` 现在是 CLI 后端。
  - 桌面壳 `RunCli` 改为调用 `LocalAreaInterconnection.Cli.exe`。
  - 删除旧 `dist/LocalAreaInterconnection.Desktop.exe`，避免入口混淆。
- 更新 `windows-cli/LocalAreaInterconnectionDesktop.cs`：
  - 新增 `Ping 目标 / Ping target` 输入框。
  - 新增“复制邀请 / Copy invite”按钮。
  - 新增“复制我的 IP / Copy my IP”按钮。
  - 新增“网络诊断 / Network diagnose”按钮，调用 `network-observe --adapter-scan true` 并带上 ping 目标、端口、网段和本机虚拟 IP。
  - 新增“导出诊断 / Export diagnostics”按钮，使用保存对话框选择 JSON 文件，并调用 `diagnostic-export`。
  - 创建房间后自动从 CLI 输出回填邀请码、虚拟网段和房主虚拟 IP。
  - 解析邀请码后自动回填虚拟网段。
  - 加入房间后自动回填虚拟网段、建议本机虚拟 IP 和房主虚拟 IP 作为 ping 目标。
  - 复制按钮会把邀请码或虚拟 IP 写入剪贴板，并在输出区显示确认信息。
  - 导出诊断时，如果界面里选择了 netsh 输出文件，会传给 `diagnostic-export --firewall-netsh-output`。
  - 动作区继续使用动态高度，新增按钮后不会固定挤压输出区。
- 更新桌面 UI 中英文文案：
  - `pingTarget`
  - `copyInvite`
  - `copyIp`
  - `networkDiagnose`
  - `exportDiagnostics`
  - `inviteCopied`
  - `ipCopied`
  - `nothingToCopy`
  - `saveDiagnosticBundle`
  - `jsonFilesFilter`
- 更新 `README.md`：
  - 明确桌面入口是 `dist\LocalAreaInterconnection.exe`。
  - 明确 CLI 后端是 `dist\LocalAreaInterconnection.Cli.exe`。
  - 补充桌面壳现在支持复制邀请码、复制虚拟 IP、网络诊断和导出诊断包。
- 本轮继续对照 `FUNCTIONAL_DESIGN.md` 的房间页/诊断页要求，推进桌面测试壳的房间详情视图：
  - 功能设计要求房间页显示房间名称、邀请码、我的虚拟 IP、成员列表、每个成员延迟、连接路径、广播状态。
  - 功能设计要求诊断页每个异常都要给下一步动作。
- 更新 `windows-cli/LocalAreaInterconnectionDesktop.cs`：
  - 将主布局调整为三列：左侧输入表单，中间操作按钮，右侧房间详情摘要。
  - 输出区横跨中间和右侧两列，保留命令输出/诊断 JSON 查看能力。
  - 新增右侧“房间详情 / Room details”面板。
  - 房间详情面板显示：
    - 房间名和虚拟网段。
    - 连接摘要。
    - 广播状态和游戏流量状态。
    - 成员/IP 摘要。
    - 下一步建议。
  - 创建房间后刷新详情为房主模式，并提示复制邀请码、启动游戏并创建 LAN 房间。
  - 解析邀请码后刷新下一步建议为加入房间。
  - 加入房间后刷新详情，显示本机建议虚拟 IP 和房主 ping 目标。
  - 网络诊断后解析 `network-observe` 的 `diagnosticSnapshot`，将 `virtual_adapter`、`tunnel`、`p2p`、`broadcast`、`game_traffic` 显示为连接/广播/游戏流量摘要。
  - 根据诊断状态输出下一步建议：
    - 虚拟网卡异常时提示检查虚拟网卡是否存在、启用并分配房间 IP。
    - tunnel/P2P 异常时提示检查 ping/P2P、换网络或端口转发。
    - 广播缺失时提示检查广播代理和游戏发现端口。
    - 游戏流量缺失时提示启动游戏并确认绑定虚拟网卡。
    - 全部健康时提示尝试进入游戏 LAN 房间。
  - 语言切换后会刷新房间详情文案。
  - 新增房间详情相关中英文文案。
- 更新 `README.md`：
  - 补充右侧房间详情面板会显示房间、虚拟网段、成员/IP、连接检查、广播/游戏流量状态和下一步建议。
- 本轮继续对照 `FUNCTIONAL_DESIGN.md` 的诊断模块要求，补齐桌面测试壳的包观测文件串联：
  - 功能设计要求诊断模块支持 UDP 收发测试、广播发现测试、端口流量检测和日志打包。
  - 进度文档上一轮建议给桌面壳增加“测试包观测文件路径”输入，便于串联 UDP/TCP/广播测试、`network-observe` 和 `diagnostic-export`。
- 更新 `windows-cli/LocalAreaInterconnectionDesktop.cs`：
  - 新增 `包观测文件 / Packet observation file` 输入框。
  - 新增“选择包文件 / Browse packets”按钮，使用保存对话框选择或创建 `packets.txt`。
  - 新增“广播测试 / Broadcast test”按钮，调用 `udp-broadcast-test`。
  - UDP 测试、TCP 测试、广播测试会在包观测文件路径存在时自动追加 `--observe-file <path>`。
  - UDP/TCP/广播测试默认使用“游戏端口”输入框中的第一个有效端口；端口为空或无效时回退到已有测试端口。
  - 网络诊断会在包观测文件存在时自动追加 `--packet-observations <path>`。
  - 导出诊断包会在包观测文件存在时自动追加 `--packet-observations <path>`。
  - 新增相关中英文文案：
    - `packetObservations`
    - `broadcastTest`
    - `browsePackets`
    - `selectPacketObservations`
- 更新 `README.md`：
  - 补充桌面壳的包观测文件字段可复用同一文件串联 UDP/TCP/广播测试、网络诊断和诊断导出。
- 本轮继续补齐桌面测试壳的“测试后自动诊断刷新”闭环：
  - UDP 测试、TCP 测试、广播测试在写入包观测文件后，会自动运行一次 `network-observe`。
  - 自动诊断会复用当前网段、本机虚拟 IP、ping 目标、游戏端口和包观测文件。
  - 自动诊断结果会刷新右侧房间详情面板。
  - 输出区会同时保留测试命令输出和自动网络诊断输出，中间用“已自动刷新网络诊断 / Network diagnostics refreshed”分隔。
  - 抽出 `RunNetworkDiagnoseAndReturn`，让手动“网络诊断”和测试后的自动刷新共用同一段诊断命令拼接逻辑。
- 更新桌面 UI 中英文文案：
  - `autoNetworkDiagnose`
- 更新 `README.md`：
  - 补充当选择包观测文件时，UDP/TCP/广播测试按钮会在测试结束后自动刷新网络诊断和房间详情。

测试结果：

- 使用系统自带 .NET Framework C# 编译器成功重新编译：
  - `dist/LocalAreaInterconnection.exe`
  - `dist/LocalAreaInterconnection.Cli.exe`
- 已启动 `dist\LocalAreaInterconnection.exe` 并确认窗口进程能保持运行，不再是双击即退出的命令行程序。
- 已执行 `dist\LocalAreaInterconnection.Cli.exe init --room-name "Friday LAN" --host Alice`，确认 CLI 后端仍能生成房间、虚拟网段、房主 IP 和邀请码。
- 已执行 `dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-scan true --ping-test 127.0.0.1 --expected-peers 1 --broadcast-ports 27015 --game-ports 27015`：
  - 本机未安装目标虚拟网卡，所以 `virtual_adapter=missing` 符合预期。
  - `ping-test 127.0.0.1` 得到 `tunnel=ok`、`p2p=ok`。
  - 未传 packet observation，所以 `broadcast=missing`、`game_traffic=missing` 符合预期。
- 已执行 `diagnostic-export` 冒烟测试，能生成 JSON bundle：
  - `BundleStatus=created`
  - `PingStatus=ok`
  - `NetworkStatus=needs-attention`，因本机没有目标虚拟网卡，符合预期。
- 已确认 `dist/` 当前只有：
  - `LocalAreaInterconnection.exe`
  - `LocalAreaInterconnection.Cli.exe`
- 已再次确认 `cargo --version` 仍不可用，Rust 工程仍无法在当前机器编译验证。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译：
  - `dist/LocalAreaInterconnection.exe`
  - `dist/LocalAreaInterconnection.Cli.exe`
- 本轮已启动 `dist\LocalAreaInterconnection.exe` 并确认窗口进程能保持运行。
- 本轮已执行 `dist\LocalAreaInterconnection.Cli.exe network-observe --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --adapter-scan true --ping-test 127.0.0.1 --expected-peers 1 --broadcast-ports 27015 --game-ports 27015`：
  - 输出仍包含 `diagnosticSnapshot.virtual_adapter`、`tunnel`、`p2p`、`broadcast`、`game_traffic` 字段，可供房间详情面板解析。
  - 本机未安装目标虚拟网卡，因此 `virtual_adapter=missing` 符合预期。
  - `ping-test 127.0.0.1` 得到 `tunnel=ok`、`p2p=ok`。
  - 未传 packet observation，因此 `broadcast=missing`、`game_traffic=missing` 符合预期。
- 已确认 `dist/` 当前仍只有：
  - `LocalAreaInterconnection.exe`
  - `LocalAreaInterconnection.Cli.exe`
- 已再次确认 `cargo --version` 仍不可用，Rust 工程仍无法在当前机器编译验证。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译：
  - `dist/LocalAreaInterconnection.exe`
  - `dist/LocalAreaInterconnection.Cli.exe`
- 本轮已启动 `dist\LocalAreaInterconnection.exe` 并确认窗口进程能保持运行。
- 本轮已再次用 CLI 模拟“测试 -> 包观测 -> 自动网络诊断刷新”依赖的底层链路：
  - `udp-loopback-test --port 39077 --message ping --observe-file <temp>` 写入 unicast packet observation。
  - `udp-broadcast-test --port 39078 --message discover --observe-file <temp>` 写入 broadcast packet observation。
  - `network-observe --packet-observations <temp> --broadcast-ports 39078 --game-ports 39077 --ping-test 127.0.0.1` 成功读回观测文件。
  - 诊断结果显示 `broadcast=seen`、`game_traffic=seen`、`tunnel=ok`、`p2p=ok`。
  - 临时包观测文件已删除。
- 已确认 `dist/` 当前仍只有：
  - `LocalAreaInterconnection.exe`
  - `LocalAreaInterconnection.Cli.exe`
- 已再次确认 `cargo --version` 仍不可用，Rust 工程仍无法在当前机器编译验证。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译：
  - `dist/LocalAreaInterconnection.exe`
  - `dist/LocalAreaInterconnection.Cli.exe`
- 本轮已启动 `dist\LocalAreaInterconnection.exe` 并确认窗口进程能保持运行。
- 本轮已用 CLI 模拟桌面“测试 -> 包观测 -> 网络诊断”链路：
  - `udp-loopback-test --port 39077 --message ping --observe-file <temp>` 写入 unicast packet observation。
  - `udp-broadcast-test --port 39078 --message discover --observe-file <temp>` 写入 broadcast packet observation。
  - `network-observe --packet-observations <temp> --broadcast-ports 39078 --game-ports 39077 --ping-test 127.0.0.1` 成功读回观测文件。
  - 诊断结果显示 `broadcast=seen`、`game_traffic=seen`、`tunnel=ok`、`p2p=ok`。
  - 临时包观测文件已删除。
- 已确认 `dist/` 当前仍只有：
  - `LocalAreaInterconnection.exe`
  - `LocalAreaInterconnection.Cli.exe`
- 已再次确认 `cargo --version` 仍不可用，Rust 工程仍无法在当前机器编译验证。

未完成：

- 桌面壳仍是轻量 WinForms 测试 UI，不是最终 Tauri/WinUI 产品 UI。
- 桌面按钮行为主要通过 CLI 输出文本/JSON 做回填，尚未使用稳定的应用状态模型。
- 新增的“网络诊断”和“导出诊断”仍基于现有只读扫描、ping 和测试观测文件能力；真实隧道服务状态、真实虚拟网卡包捕获、真实广播转发仍未接入。
- 包观测文件当前仍由测试命令生成，不能代表真实游戏进程或真实虚拟网卡流量。
- 桌面壳只会在包观测文件已经存在时把它传给网络诊断/诊断导出；如果只选择了路径但尚未运行测试，诊断会跳过该文件，避免读取不存在文件时报错。
- 测试后自动刷新房间详情仍然依赖 CLI JSON 文本解析，尚未有稳定应用状态模型。
- 右侧房间详情面板当前显示的是本地测试壳状态和 CLI 诊断摘要，不是真实房间成员服务，也没有真实在线状态/连接路径数据。
- 成员列表当前仍是单行摘要，不支持多成员、延迟列表、踢人、退出房间或房主关闭房间。
- 复制邀请码、复制虚拟 IP、保存诊断包的 UI 行为已通过编译和启动验证覆盖基础可用性，但未做自动化 UI 点击测试。
- Rust 原生工程已在当前机器通过编译和测试；后续重点从“能否编译”转为“迁移 C# 测试壳能力并补集成测试”。

下一步建议：

- 继续把桌面壳从“命令按钮集合”推进到更接近产品流程的房间详情页：显示房间 ID、我的虚拟 IP、邀请码、推荐操作和连接状态。
- 为桌面壳增加更稳定的本地应用状态模型，避免长期依赖从 CLI JSON 文本中临时提取字段。
- 继续为桌面壳增加更稳定的本地应用状态模型，避免长期依赖从 CLI JSON 文本中临时提取字段。
- 后续可把测试后自动诊断的摘要以更友好的方式显示，而不是在输出区追加完整 JSON。
- 后续房间详情应接入真实成员状态、连接路径、延迟、广播状态和退出/关闭房间操作。
- 继续为 Rust CLI 增加集成测试，固定关键命令的 JSON 输出 schema。
- 将当前 C# 测试壳里跑通的桌面流程、诊断导出 schema 和采集入口逐步迁回 Rust 原生核心或 Windows native service 边界。

### 2026-06-10 本次会话

已完成：

- 读取 `IMPLEMENTATION_PROGRESS.md`，确认当前阻塞是 Rust 工具链缺失、真实网络实验未开始、Git 仓库未初始化。
- 对照 `FUNCTIONAL_DESIGN.md`，选择继续补充游戏端口规则、广播策略、防火墙预案和诊断计划能力。
- 删除早期脚本原型代码、测试文件和旧脚本工程配置，避免继续偏离 Windows 桌面端 + Rust 原生核心方向。
- 新增 `native/crates/lai-core/src/game_network_plan.rs`：
  - 根据游戏模板和虚拟网段生成防火墙入站规则建议。
  - 根据游戏发现方式生成 UDP 广播转发策略。
  - 输出 Direct IP、广播、游戏流量、防火墙等诊断检查项。
  - 对未知端口、Direct IP-only、低兼容性目标给出风险提示。
- 新增 `native/crates/lai-core/src/firewall_plan.rs`：
  - 根据游戏网络计划里的端口规则生成 Windows 防火墙 dry-run 计划。
  - 输出 `netsh advfirewall firewall add rule` 命令预览。
  - 输出对应的删除/回滚命令。
  - 标记是否需要管理员权限，并在缺少端口或未绑定程序路径时给出提示。
- 更新 `native/crates/lai-core/src/lib.rs`，导出游戏网络计划和防火墙 dry-run 计划能力。
- 更新 `native/crates/lai-cli/src/main.rs`，新增 `game-plan` 和 `firewall-plan` 子命令。
- 对新增 Rust 代码做静态自查，收紧枚举匹配和测试里的 IP 类型推断，降低后续编译风险。
- 更新 `README.md` 和 `docs/ARCHITECTURE.md`，明确当前只保留 Rust 原生核心方向。
- 继续新增 `native/crates/lai-core/src/firewall_diagnostics.rs`：
  - 定义 Windows 防火墙规则观测结构。
  - 根据期望规则和观测规则输出逐条诊断结果。
  - 支持识别缺失、禁用、动作错误、方向错误、远端范围错误、程序路径不匹配。
  - 为每个异常输出下一步动作。
- 更新 `native/crates/lai-core/src/lib.rs`，导出防火墙诊断能力。
- 更新 `native/crates/lai-cli/src/main.rs`，新增 `firewall-diagnose` 子命令，可用 `--observed udp:7777,tcp:7777` 预览诊断结果。
- 更新 `README.md` 和 `docs/ARCHITECTURE.md`，补充防火墙诊断能力说明。
- 新增 `native/crates/lai-core/src/windows_firewall_parser.rs`，可解析英文 `netsh advfirewall firewall show rule name=all` 输出为防火墙规则观测数据。
- 新增 `native/crates/lai-core/src/virtual_adapter_plan.rs`，可生成 Windows 虚拟网卡 IP、MTU、接口 metric 的 dry-run 配置计划。
- 更新 Rust CLI：
  - `firewall-diagnose` 支持 `--netsh-output <file>`。
  - 新增 `adapter-plan` 子命令。
- 新增 `windows-cli/LocalAreaInterconnectionCli.cs`，并编译为 `dist/LocalAreaInterconnection.exe`：
  - 房间流程：`init`、`decode`、`join`。
  - 计划生成：`game-plan`、`adapter-plan`、`firewall-plan`。
  - 诊断：`diagnose`、`adapter-diagnose`、`adapter-scan`、`firewall-diagnose`、`firewall-scan`。
  - 防火墙规则执行入口：`firewall-apply`、`firewall-remove`，默认 dry-run，必须 `--yes true` 才执行。
  - 虚拟网卡配置执行入口：`adapter-apply`，默认 dry-run，必须 `--yes true` 才执行。
  - 网络实验：`udp-loopback-test`、`udp-listen`、`udp-send`、`tcp-loopback-test`。
- 新增 `windows-cli/LocalAreaInterconnectionDesktop.cs`，并编译为 `dist/LocalAreaInterconnection.Desktop.exe`：
  - 提供创建房间、邀请码解析/加入、网卡计划/扫描、游戏计划、防火墙计划/诊断/扫描、通用诊断、UDP/TCP 测试按钮。
- 新增 `tools/GenerateIcon.cs`，生成 `assets/LocalAreaInterconnection.ico` 和 `assets/LocalAreaInterconnection.preview.png`。
- 已将图标嵌入 `dist/LocalAreaInterconnection.exe` 与 `dist/LocalAreaInterconnection.Desktop.exe`。
- 更新桌面测试壳视觉：
  - 薄雾蓝背景。
  - 柔和微光。
  - 动态粒子动画，使用低频刷新和双缓冲降低闪烁。
  - 蓝色发光按钮和深蓝输入框样式。
  - Browse netsh 和 Copy output 测试辅助按钮。
- 继续修正桌面测试壳：
  - 去除流光效果。
  - 修复系统白色标题栏，改为统一深蓝自定义标题栏。
  - 修复输入框被撑高的问题，固定表单行高。
  - 修复按钮区高度不足的问题，按钮区根据窗口宽度自动调整列数、按钮宽度和区域高度。
  - 将“输出”改为“命令输出 / 诊断结果”，并加入初始说明。
  - 语言下拉框改成深色自绘样式。
  - 最小化、最大化/还原、关闭按钮增加悬停提示。
  - 无边框窗口支持从边缘和角落拖拽缩放。
- 新增桌面 UI 国际化：
  - 启动时根据 Windows UI 语言选择中文或英文。
  - 标题栏提供 `English / 中文` 语言切换。
  - 用户语言选择保存到 `%APPDATA%\LocalAreaInterconnection\settings.lang`。
  - 当前覆盖桌面标签、按钮、文件选择对话框和缺少 CLI 的提示。
- 更新 `README.md`，补充当前可运行 Windows CLI 和桌面测试壳命令。
- 本轮继续新增 `native/crates/lai-core/src/network_observation.rs`：
  - 定义网卡观测、隧道观测、包观测、网络观测报告结构。
  - 将虚拟网卡启用/IP 匹配、隧道连接/丢包、P2P peer 数、广播包、游戏流量观测转换为统一诊断快照。
  - 复用已有 `evaluate_diagnostics` 输出用户可理解的问题和下一步动作。
  - 增加健康样例和异常样例单元测试草案。
- 更新 `native/crates/lai-core/src/ip.rs`：
  - 为 `Ipv4Subnet` 增加字符串格式 serde 支持，避免未来 JSON 输出变成 `{ network, prefix }` 对象而破坏现有 `10.77.12.0/24` 表达。
- 更新 `native/crates/lai-core/src/lib.rs`，导出网络观测相关类型和评估函数。
- 更新 `native/crates/lai-cli/src/main.rs`：
  - 新增 `network-observe` 子命令。
  - 支持手动输入 adapter/tunnel/peer/packet 观测样例并输出合并诊断报告。
- 更新 `windows-cli/LocalAreaInterconnectionCli.cs`：
  - 新增可运行的 `network-observe` 命令。
  - 支持健康和异常观测样例输出 JSON 诊断报告。
- 更新 `README.md` 和 `docs/ARCHITECTURE.md`：
  - 补充网络观测边界说明。
  - 将下一步从“定义观测边界”推进为“接入真实采集层”。
- 本轮继续新增 `native/crates/lai-core/src/windows_adapter_parser.rs`：
  - 解析英文 `netsh interface ipv4 show config name=<adapter>` 输出。
  - 提取虚拟网卡 IP、子网前缀、接口 metric、启用状态等观测字段。
  - 生成 `AdapterObservation`，用于接入 `network_observation` 统一诊断报告。
  - 增加英文 netsh 样例和空输出单元测试草案。
- 更新 `native/crates/lai-core/src/lib.rs`，导出 `parse_netsh_adapter_observation`。
- 更新 `native/crates/lai-cli/src/main.rs`：
  - `network-observe` 新增 `--adapter-netsh-output <file>`。
  - 支持从网卡 netsh 输出文件构造 adapter observation，不再只能手动传 `--assigned-ip`。
- 更新 `windows-cli/LocalAreaInterconnectionCli.cs`：
  - `network-observe` 新增 `--adapter-netsh-output <file>`。
  - `network-observe` 新增 `--adapter-scan true`，会只读执行 `netsh interface ipv4 show config name=<adapter>` 并合并诊断。
  - 输出新增 `adapterObservation`，包含来源 `manual`、`netsh-file` 或 `netsh-scan`、解析到的 IP/子网/metric，以及扫描错误。
  - 修正扫描失败时的状态归类，从容易误导的 `ip-mismatch` 改为 `missing`。
- 更新 `README.md`、`docs/ARCHITECTURE.md` 和 `docs/M1_NETWORK_EXPERIMENT.md`：
  - 补充 `network-observe --adapter-netsh-output` 与 `--adapter-scan true` 用法。
  - 将架构下一步推进为采集隧道状态、广播/游戏包观测，并在后续服务层替换文本解析为 Windows API 采集。
- 本轮继续新增 `native/crates/lai-core/src/packet_observation_parser.rs`：
  - 定义统一 packet observation 文本行解析。
  - 格式为 `protocol:source_ip:destination_ip:port:broadcast|unicast:direction:bytes`。
  - 支持从多行文本解析 UDP/TCP 包观测，用于 `network_observation` 判断广播和游戏端口流量。
  - 增加 UDP/TCP 样例单元测试草案。
- 更新 `native/crates/lai-core/src/lib.rs`，导出 `parse_packet_observation_line` 和 `parse_packet_observation_lines`。
- 更新 `native/crates/lai-cli/src/main.rs`：
  - `network-observe` 新增 `--packet-observations <file>`。
  - 原有 `--packets` 内联参数改为复用核心 packet observation 解析器。
- 更新 `windows-cli/LocalAreaInterconnectionCli.cs`：
  - `network-observe` 新增 `--packet-observations <file>`，可读取由测试命令或未来采集器生成的包观测文件。
  - `udp-loopback-test`、`udp-listen`、`udp-send`、`tcp-loopback-test` 新增 `--observe-file <file>`，会把测试中观察到的 UDP/TCP 包追加为统一 packet observation 行。
  - 保留原有 `--packets` 内联输入，并与 `--packet-observations` 文件输入合并。
- 更新 `README.md`、`docs/ARCHITECTURE.md` 和 `docs/M1_NETWORK_EXPERIMENT.md`：
  - 补充 `--packet-observations` 和 `--observe-file` 用法。
  - 将下一步推进为把测试生成的包观测文件替换/扩展为真实广播和游戏包捕获摘要。
- 本轮继续新增 `native/crates/lai-core/src/windows_ping_parser.rs`：
  - 解析 Windows `ping` 输出中的发送、接收、丢失、平均延迟。
  - 生成 `TunnelObservation`，用于 `network_observation` 判断 tunnel/P2P 是否健康。
  - 增加成功 ping 和失败 ping 的单元测试草案。
- 更新 `native/crates/lai-core/src/lib.rs`，导出 `parse_windows_ping_observation`。
- 更新 `native/crates/lai-cli/src/main.rs`：
  - `network-observe` 新增 `--ping-output <file>`。
  - 可从 ping 输出文件生成 tunnel observation，不再只能手动传 `--tunnel-state`、`--latency-ms`、`--packet-loss-percent`。
- 更新 `windows-cli/LocalAreaInterconnectionCli.cs`：
  - `network-observe` 新增 `--ping-output <file>`，可导入 ping 输出文本。
  - `network-observe` 新增 `--ping-test <host>`，使用 .NET `Ping` API 执行只读连通性测试，统计成功次数、平均延迟和丢包率。
  - 输出新增 `tunnelObservation`，包含状态、连接 peer 数、期望 peer 数、延迟、丢包率、来源和 ping host。
  - 修正早期用命令行 `ping` 文本解析导致本机 `127.0.0.1` 成功 ping 被误判为 disconnected 的问题：实际 ping 测试改用 .NET Ping API，`--ping-output` 仍保留文本导入。
- 更新 `README.md`、`docs/ARCHITECTURE.md` 和 `docs/M1_NETWORK_EXPERIMENT.md`：
  - 补充 `network-observe --ping-test` 和 `--ping-output` 用法。
  - 将架构下一步推进为未来用真实 tunnel service 状态替换 ping 派生状态。
- 本轮继续更新 `windows-cli/LocalAreaInterconnectionCli.cs`：
  - 新增 `udp-broadcast-test` 命令。
  - 向 `255.255.255.255:<port>` 发送 UDP 广播并在本机监听接收。
  - 支持 `--observe-file <file>`，收到广播后追加 `broadcast` 类型 packet observation 行。
  - 保持与 `network-observe --packet-observations` 的统一格式，便于后续替换为真实虚拟网卡广播捕获摘要。
- 更新 `README.md` 和 `docs/M1_NETWORK_EXPERIMENT.md`：
  - 补充 `udp-broadcast-test --observe-file` 用法。
  - 在 M1 checklist 中加入广播观测文件链路检查项。
- 本轮继续更新 `windows-cli/LocalAreaInterconnectionCli.cs`：
  - 新增 `diagnostic-export` 命令。
  - 通过 `--out <file>` 写出只读 JSON 诊断包。
  - 诊断包包含环境元数据、输入摘要、网卡扫描与诊断、防火墙扫描与诊断、ping 派生 tunnel 观测、packet observation 摘要和综合 `networkObservation`。
  - 默认不修改 Windows 防火墙、网卡或路由，仅执行只读扫描和本地 ping。
  - 在缺少 `--expected-ip`/`--subnet` 时，网卡诊断区块输出 `missing-input`，避免把缺少输入误判为系统状态。
  - 在缺少 `--ping-test`/`--ping-output` 时，ping 区块输出 `skipped`，避免把“未测试”误判为“正常”。
  - 诊断包顶层增加 `status: created`，便于后续自动收集和解析。
- 本轮修正 Windows 测试程序防火墙解析：
  - `firewall-diagnose --netsh-output` 支持中文 `netsh advfirewall firewall show rule name=all` 输出。
  - 支持按中文 `规则名称` 正确分块。
  - 支持中文字段 `协议`、`本地端口`、`已启用`。
  - 支持中文启用值 `是`，禁用值会被识别为 disabled。
- 更新 `README.md`：
  - 补充 `diagnostic-export` 示例命令。
  - 说明导出包包含本机网卡和 Windows 防火墙配置，分享前应检查内容。
- 更新 `docs/ARCHITECTURE.md`：
  - 补充 Windows 测试 CLI 的只读诊断导出包能力。
  - 将后续建议更新为 Rust 工具链可用后把诊断包生成迁移到 Rust/native service 边界。
- 更新 `docs/M1_NETWORK_EXPERIMENT.md`：
  - 在 checklist 中加入 `diagnostic-export --out <file>`。
  - 在 M1 常用命令中加入实验结束后导出诊断包的命令。

测试结果：

- 已确认 `cargo --version` 仍不可用，当前机器没有 Rust 工具链。
- 因当时缺少 `cargo`/`rustc`，本轮 Rust 代码尚未编译验证；该阻塞已在 2026-06-11 解除。
- 已通过文件扫描确认早期脚本代码文件和旧路线关键词均已清理。
- 已再次确认旧路线关键词无命中，`native/` 下新增 `firewall_diagnostics.rs`。
- 使用系统自带 .NET Framework C# 编译器成功生成：
  - `dist/LocalAreaInterconnection.exe`
  - `dist/LocalAreaInterconnection.Desktop.exe`
- 已验证图标文件生成成功，并生成 PNG 预览；`view_image` 可打开 `assets/LocalAreaInterconnection.preview.png`。
- 已多次重新编译并覆盖 `dist/LocalAreaInterconnection.Desktop.exe`，当前版本包含国际化、统一深色标题栏、自适应按钮区、动态粒子、深色语言下拉框和边缘缩放。
- 已确认 `dist/LocalAreaInterconnection.exe` 存在，并通过 `diagnose --virtual-adapter ok --firewall allowed --p2p ok --broadcast missing` 冒烟测试。
- 已执行并通过以下 Windows CLI 冒烟测试：
  - `help`
  - `init`
  - `decode`
  - `join`
  - `adapter-plan`
  - `adapter-apply` 无确认 dry-run
  - `adapter-diagnose` 样例文件解析
  - `adapter-scan` 本机只读扫描，未找到虚拟网卡时能给出明确失败项
  - `diagnose`
  - `game-plan`
  - `firewall-plan`
  - `firewall-apply` 无确认 dry-run
  - `firewall-remove` 无确认 dry-run
  - `firewall-diagnose` observed 简写
  - `firewall-diagnose --netsh-output` 样例文件解析，能区分 present 与 disabled
  - `firewall-scan` 本机只读扫描
  - `udp-loopback-test`
  - `udp-listen` 与 `udp-send` 本机联动
  - `tcp-loopback-test`
- 本轮已使用系统自带 .NET Framework C# 编译器重新编译并覆盖 `dist/LocalAreaInterconnection.exe`。
- 本轮已执行并通过以下新增 Windows CLI 冒烟测试：
  - `help` 中能看到 `network-observe`。
  - `network-observe` 健康样例：网卡 IP 匹配、隧道 connected、peer 数满足、广播包和游戏流量均可观测，输出 `status: ok`。
  - `network-observe` 异常样例：网卡 IP 不匹配、隧道 disconnected、P2P failed、广播/游戏流量缺失，输出 `status: needs-attention`，并生成 5 个诊断问题。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译并覆盖 `dist/LocalAreaInterconnection.exe`。
- 本轮已执行并通过以下新增 Windows CLI 冒烟测试：
  - `network-observe --adapter-netsh-output <sample>`：从英文 netsh 样例解析到 `assignedIp=10.77.12.2`、`observedSubnet=10.77.12.0/24`、`interfaceMetric=5`，输出 `status: ok` 和 `adapterObservation.source=netsh-file`。
  - `network-observe --adapter-scan true`：本机不存在 `LocalAreaInterconnection` 虚拟网卡时不崩溃，输出 `virtual_adapter: missing`，并在 `adapterObservation.scanError` 保留 `netsh exited with 1`。
  - `help` 中能看到 `--adapter-netsh-output` 和 `--adapter-scan true` 用法。
- 已确认测试用临时样例文件 `%TEMP%\lai-adapter-netsh-sample.txt` 已删除。
- 当时已再次确认 `cargo --version` 仍不可用，Rust 新增解析器和 Rust CLI 变更尚未编译验证；该阻塞已在 2026-06-11 解除。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译并覆盖 `dist/LocalAreaInterconnection.exe`。
- 本轮已执行并通过以下新增 Windows CLI 冒烟测试：
  - `udp-loopback-test --observe-file <temp>`：本机 UDP loopback 成功，并写出 `udp:127.0.0.1:127.0.0.1:39077:unicast:inbound:4` packet observation 行。
  - `network-observe --packet-observations <temp>`：读取上述观测文件后能识别 `game_traffic: seen`。
  - `help` 中能看到 `--packet-observations` 和 UDP/TCP 测试命令的 `--observe-file` 用法。
- 已确认测试用临时包观测文件 `%TEMP%\lai-packets-observe.txt` 已删除。
- 当时已再次确认 `cargo --version` 仍不可用，Rust 新增 packet observation 解析器和 Rust CLI 变更尚未编译验证；该阻塞已在 2026-06-11 解除。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译并覆盖 `dist/LocalAreaInterconnection.exe`。
- 本轮已执行并通过以下新增 Windows CLI 冒烟测试：
  - `network-observe --ping-test 127.0.0.1 --expected-peers 1`：通过 .NET Ping API 得到 `tunnel: ok`、`p2p: ok`、`latencyMs=0`、`packetLossPercent=0`。
  - `network-observe --ping-output <sample>`：从失败 ping 样例导入得到 `tunnel: down`、`p2p: failed`、`packetLossPercent=100`。
  - `help` 中能看到 `--ping-test` 和 `--ping-output` 用法。
- 已确认测试用临时 ping 输出文件 `%TEMP%\lai-ping-failed.txt` 已删除。
- 当时已再次确认 `cargo --version` 仍不可用，Rust 新增 ping 解析器和 Rust CLI 变更尚未编译验证；该阻塞已在 2026-06-11 解除。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译并覆盖 `dist/LocalAreaInterconnection.exe`。
- 本轮已执行并通过以下新增 Windows CLI 冒烟测试：
  - `udp-broadcast-test --port 39078 --message discover --observe-file <temp>`：本机收到 UDP 广播，输出 `status: ok`，并写出 `udp:<local-ip>:255.255.255.255:39078:broadcast:inbound:8` packet observation 行。
  - `network-observe --packet-observations <temp> --broadcast-ports 39078`：读取上述观测文件后能识别 `broadcast: seen`。
  - `help` 中能看到 `udp-broadcast-test --observe-file` 用法。
- 已确认测试用临时广播观测文件 `%TEMP%\lai-broadcast-packets.txt` 已删除。
- 已再次确认 `cargo --version` 仍不可用，本轮仅更新 Windows C# 测试壳和文档。
- 本轮已再次使用系统自带 .NET Framework C# 编译器重新编译并覆盖 `dist/LocalAreaInterconnection.exe`。
- 本轮已执行并通过以下新增 Windows CLI 冒烟测试：
  - `help` 中能看到 `diagnostic-export` 示例命令。
  - `diagnostic-export --out <temp> --adapter-name LocalAreaInterconnection --expected-ip 10.77.12.2 --subnet 10.77.12.0/24 --ping-test 127.0.0.1 --expected-peers 1 --packet-observations <temp> --broadcast-ports 39078 --game-ports 39077 --game-name "Example Game" --ports 39077,39078`：成功生成 JSON bundle，命令输出 `status=ok`，bundle 顶层 `status=created`。
  - 上述完整导出中，本机未安装目标虚拟网卡，`adapterScan.status=needs-attention`、`networkObservation.status=needs-attention`，符合预期。
  - 上述完整导出中，`ping.status=ok`，packet observation 摘要识别到 `broadcastCount=1` 和 `gameTrafficCount=1`。
  - `diagnostic-export --out <temp>` 最小参数导出不崩溃，bundle 顶层 `status=created`，`ping.status=skipped`，网卡诊断输出 `missing-input`。
  - `firewall-diagnose --netsh-output <中文样例>` 能正确识别中文 Windows 防火墙输出：UDP 规则为 `present`，TCP 规则为 `disabled`，`problemCount=1`。
- 已确认本轮测试用临时诊断包、packet observation 文件和中文防火墙样例文件已删除。
- 当时再次确认 `cargo --version` 仍不可用，Rust 新增/既有代码仍未编译验证；该阻塞已在 2026-06-11 解除。

未完成：

- 当时 Rust 原生工程仍未编译验证，因为机器没有 `cargo`/`rustc`；该阻塞已在 2026-06-11 解除。
- 当前新增能力仍主要是“规则、诊断计划、防火墙命令预览、网络观测建模、Windows netsh 文本解析、测试命令生成的包观测文件、ping 派生连通性观测、本机 UDP 广播测试”，尚未实际调用 Windows 防火墙 API、虚拟网卡驱动、隧道服务或真实虚拟网卡包捕获。
- `diagnostic-export` 当前位于 Windows C# 测试壳中，属于当时无 Rust 工具链时的可运行验证入口；后续应迁回 Rust 原生核心或 Windows native service 边界。
- `diagnostic-export` 会包含本机网卡和 Windows 防火墙原始输出，适合测试组内部排障；对外分享前需要人工检查隐私和本地配置内容。
- `game_network_plan.rs`、`firewall_plan.rs`、`firewall_diagnostics.rs`、`network_observation.rs`、`windows_adapter_parser.rs`、`packet_observation_parser.rs`、`windows_ping_parser.rs` 和 `lai-cli` 新命令当时需要安装 Rust 后执行 `cargo test` 和 CLI 手动验证；基础编译测试已在 2026-06-11 完成。
- `windows-cli/` 是当前无 Rust 工具链环境下的 Windows 可执行测试壳，不应替代长期 Rust 原生核心。
- `firewall-apply`、`firewall-remove`、`adapter-apply` 只有显式 `--yes true` 才执行；需要管理员终端，尚未在管理员权限下做真实系统修改测试。
- 当前桌面壳是轻量测试 UI，不是最终产品 UI。

下一步建议：

- 安装 Rust stable 后先在 `native/` 执行 `cargo test`，修正任何编译问题。
- 继续补 M1/M3 相关的真实采集层：Windows 网卡状态已有 netsh 文本解析入口，UDP/TCP/广播测试已可生成包观测文件，ping 已可生成 tunnel/P2P 观测；下一步应接入真实 tunnel service 状态和真实虚拟网卡广播/游戏包捕获摘要。
- 继续补 M3 相关的 Windows 防火墙采集层：先把真实系统规则转换成 `FirewallRuleObservation`，再考虑实际添加规则。
- 将 `diagnostic-export` 的 JSON schema 固化，并在 Rust 工具链可用后迁移到 `lai-core`/native service，避免 Windows C# 测试壳成为长期实现。
- 后续桌面端优先选择 Tauri 或原生 Windows UI 壳，核心能力继续放在 Rust。
- 下一轮优先：
  - 安装 Rust 工具链并验证 `native/`。
  - 将 Windows 测试壳中已经跑通的 UDP/TCP/广播测试、adapter scan/apply、firewall scan/apply、network-observe、adapter netsh parsing、packet observation file、ping observation 能力逐步迁回 Rust 原生核心或 Windows 服务层。
  - 将 Windows 测试壳中已经跑通的 `diagnostic-export` 迁回 Rust 原生核心或 Windows 服务层，并为导出包字段增加测试。
  - 在真实管理员终端中测试防火墙添加/删除和指定虚拟网卡配置。

### 历史进展

已完成：

- 曾短暂落地脚本原型用于行为参考，但已在本次会话删除，后续不再作为实施方向。
- 新增 `native/crates/lai-core`：Rust 原生核心骨架，覆盖 IP/子网、房间、邀请码、加入计划、诊断、广播策略、游戏模板。
- 新增 `native/crates/lai-cli`：Rust CLI 骨架，供未来安装 Rust 工具链后验证核心行为。
- 补充工程架构和 M0/M1 调研模板文档。
- 更新 README 和架构文档，明确最终方向是 Windows 客户端 + Rust 原生核心。

未完成：

- 初始化 Git 仓库。
- 真实网络实验和 Windows 虚拟网卡验证。
- UI 层、隧道层和防火墙自动化能力。
- Rust 原生工程当时尚未编译验证：当时机器没有 `cargo`/`rustc`；该阻塞已在 2026-06-11 解除。

## 基线假设

以下假设用于启动实施计划；若后续确认有变更，应在“决策记录”中补充新决策。

- 第一版只支持 Windows。
- 第一阶段先做调研验证和网络 PoC，不直接承诺完整二层虚拟 LAN。
- 最终客户端核心优先使用 Rust 原生实现，避免把虚拟网卡、隧道、广播代理等高频网络路径放在脚本运行时中。
- 推荐优先验证路线 B 或路线 C：
  - 路线 B：封装 ZeroTier/SoftEther 等成熟方案，先验证用户流程、诊断和游戏适配。
  - 路线 C：用 WireGuard 思路做三层虚拟网络，再补 UDP 广播代理。
- 正式产品默认接受轻量协调服务，用于临时房间、候选地址交换、NAT 打洞和一次性密钥材料交换。
- MVP 暂不内置中继兜底；P2P 失败时先提供明确诊断、端口转发建议或网络切换建议。
- 安全默认值为房间隔离、端到端加密、不暴露真实局域网、不转发物理 LAN 广播。

## 待确认问题

- [ ] 第一批适配测试的 Steam 游戏列表。
- [ ] MVP 采用路线 B、路线 C，还是直接进入自研轻量隧道。
- [ ] 是否允许安装第三方虚拟网卡或网络组件。
- [ ] 是否接受轻量协调服务作为默认体验的一部分。
- [ ] 是否需要从 MVP 开始预留中继接口，即使第一版不提供公共中继。
- [ ] Windows 驱动、签名、安装权限和杀软误报是否纳入第一阶段风险验证。
- [ ] 是否需要 UI 原型先行，还是网络 PoC 先行。

## 总体任务计划

### 阶段 0：项目基建

目标：让仓库具备可持续推进的基本结构。

任务：

- [x] 初始化 Git 仓库。
- [ ] 确定最终工程技术栈：当前目标为 Windows 客户端 + Rust 原生核心；桌面 UI 壳仍待确认。
- [x] 建立第一版目录结构：核心域模型、CLI、文档、测试记录模板。
- [x] 添加基础 README，说明项目目标、非目标、开发环境和当前阶段。
- [x] 添加变更记录或决策记录入口。

完成标准：

- 新成员能根据 README 搭起开发环境。
- 关键技术选择有记录。
- 实验代码和正式代码边界清楚。

### 阶段 1：M0 调研验证

目标：确认需求是否真实、目标游戏的 LAN 行为是什么、现有工具能覆盖到什么程度。

任务：

- [ ] 收集 10 款候选 LAN 游戏。
- [ ] 记录每款游戏是否支持 LAN 列表、Direct IP、固定端口、广播或组播发现。
- [ ] 对比 Radmin VPN、ZeroTier、SoftEther 在这些游戏上的效果。
- [ ] 记录失败案例：能 ping 但看不到房间、广播无效、防火墙阻止、游戏绑定错误网卡。
- [ ] 输出第一版技术选型说明。

完成标准：

- 至少 5 款游戏有明确测试记录。
- 能判断“快速封装成熟方案”是否足以支撑 MVP。
- 能列出前 3 个最常见联机失败原因。

### 阶段 2：M1 本机虚拟网络实验

目标：在两台 Windows 机器之间验证虚拟 IP 互通、UDP 单播和广播捕获/转发。

任务：

- [ ] 搭建两台 Windows 测试环境。
- [ ] 验证虚拟网卡安装、启用、禁用和 IP 分配。
- [ ] 验证两端虚拟 IP ping 通。
- [ ] 验证 UDP 单播收发。
- [ ] 捕获 UDP 广播包并记录端口、频率、来源进程。
- [ ] 实现或验证最小广播转发链路。
- [ ] 记录延迟、丢包、CPU 占用和断线表现。

完成标准：

- 两台机器虚拟 IP 稳定互通 30 分钟。
- 有 UDP 单播和 UDP 广播测试日志。
- 明确当前路线是否支持游戏房间自动发现。

### 阶段 3：M2 房间原型

目标：做出可运行的房间流程原型。

任务：

- [ ] 创建房间并生成房间 ID。
- [ ] 生成长邀请码，包含房间 ID、虚拟网段、主机节点、连接信息和 join token。
- [ ] 加入房间并解析邀请码。
- [ ] 分配虚拟 IP。
- [ ] 展示成员列表、虚拟 IP、在线状态和连接路径。
- [ ] 支持退出房间和房主关闭房间。

完成标准：

- 两台机器能通过邀请码进入同一房间。
- UI 或 CLI 能显示双方虚拟 IP 和连接状态。
- 房间生命周期有基本日志。

### 阶段 4：M3 游戏联机 MVP

目标：支持至少 2 款 LAN 游戏完成异地虚拟局域网联机。

任务：

- [ ] 支持 Direct IP 方式加入。
- [ ] 支持基础 UDP 广播转发。
- [ ] 支持基础 TCP 转发或确认目标游戏不依赖 TCP。
- [ ] 添加手动游戏端口规则。
- [ ] 实现防火墙诊断：客户端规则、游戏进程入站规则、专用网络提示。
- [ ] 实现连接诊断：P2P、延迟、丢包、虚拟网卡、互 ping、广播流量、游戏端口流量。
- [ ] 建立游戏兼容性表。

完成标准：

- 至少 2 款游戏完成真实联机测试。
- 联机失败时能给出具体下一步动作，而不是只显示失败。
- 用户能复制邀请码、加入房间、复制虚拟 IP。

### 阶段 5：M4 小范围测试

目标：收集真实网络环境下的成功率、失败原因和体验问题。

任务：

- [ ] 邀请 5-20 名用户测试。
- [ ] 统计房间创建成功率、P2P 成功率、广播发现成功率、断线次数。
- [ ] 收集失败日志并按原因分类。
- [ ] 整理常见问题和诊断文案。
- [ ] 决定是否进入自研隧道、中继、账号或商业化能力。

完成标准：

- 有测试报告。
- 有下一阶段优先级排序。
- 能判断 MVP 是否值得继续投入。

## 里程碑进度

### M0：调研验证

状态：未开始

计划：

- [ ] 收集目标游戏列表。
- [ ] 测试现有工具：Radmin VPN、ZeroTier、SoftEther。
- [ ] 记录每个游戏的 LAN 发现方式。
- [ ] 明确 MVP 技术路线。

记录：

```text
2026-06-10：已从功能设计文档提炼 M0 任务和完成标准，尚未开始实际调研。
```

### M1：本机虚拟网络实验

状态：未开始

计划：

- [ ] 两台 Windows 机器虚拟 IP 互通。
- [ ] UDP 单播测试。
- [ ] UDP 广播测试。
- [ ] 基础延迟和丢包测试。

记录：

```text
2026-06-10：已明确 M1 应先验证虚拟网卡、虚拟 IP、UDP 单播、广播捕获/转发和稳定性。
2026-06-10：已新增网络观测模型，可先接收手动/测试壳输入的网卡、隧道、广播包、游戏流量观测并生成诊断报告；尚未接入真实包捕获或隧道服务。
2026-06-10：已新增 Windows 网卡 netsh 输出解析，并接入 `network-observe --adapter-netsh-output` 与 `--adapter-scan true`；当前可从真实/样例网卡配置文本生成 adapter observation。
2026-06-10：已新增 packet observation 文件格式，并让 UDP/TCP 测试命令可通过 `--observe-file` 追加观测行；`network-observe --packet-observations` 可读取文件并识别游戏端口流量。
2026-06-10：已新增 ping 连通性观测，`network-observe --ping-test <host>` 可通过 .NET Ping API 生成延迟、丢包和 tunnel/P2P 状态；`--ping-output <file>` 可导入 ping 文本样例。
2026-06-10：已新增 `udp-broadcast-test --observe-file`，可在本机广播测试中生成 broadcast packet observation，并通过 `network-observe --packet-observations` 识别 `broadcast: seen`；真实虚拟网卡广播捕获尚未接入。
2026-06-10：已新增 `diagnostic-export --out <file>`，可把 M1 实验后的网卡、防火墙、ping、packet observation 和综合网络诊断打包为只读 JSON；真实隧道服务状态和真实虚拟网卡包捕获仍未接入。
2026-06-11：桌面测试壳已新增包观测文件路径输入，UDP/TCP/广播测试可写入同一观测文件，网络诊断和诊断导出可复用该文件；真实虚拟网卡包捕获仍未接入。
2026-06-11：桌面测试壳在包观测文件存在时，UDP/TCP/广播测试后会自动运行 `network-observe` 并刷新房间详情；真实虚拟网卡包捕获仍未接入。
```

### M2：房间原型

状态：未开始

计划：

- [ ] 创建房间。
- [ ] 加入房间。
- [ ] 生成邀请码。
- [ ] 显示成员和虚拟 IP。

记录：

```text
2026-06-10：已明确 M2 的核心是邀请码、成员状态、虚拟 IP 分配和房间生命周期。
2026-06-11：桌面测试壳已支持创建房间后自动回填邀请码/虚拟网段/房主虚拟 IP，支持复制邀请码和复制虚拟 IP，加入房间后可回填建议本机虚拟 IP 和房主 ping 目标；这仍是本地测试壳流程，不是真实房间生命周期服务。
2026-06-11：桌面测试壳已新增右侧房间详情摘要，可显示房间、网段、成员/IP、连接状态、广播/游戏流量状态和下一步建议；成员状态仍来自本地表单与诊断摘要，尚未接入真实房间成员服务。
```

### M3：游戏联机 MVP

状态：未开始

计划：

- [ ] 至少支持 2 款 LAN 游戏成功联机。
- [ ] 支持 Direct IP。
- [ ] 支持基础广播转发。
- [ ] 支持防火墙诊断。

记录：

```text
2026-06-10：已明确 M3 不只要求网络互通，还要求可诊断、可解释、可给用户下一步动作。
2026-06-10：已新增 network-observe 诊断入口，能把 P2P、广播、游戏流量观测转为用户下一步动作；真实联机和真实流量采集尚未开始。
2026-06-10：已接入测试生成的包观测文件，可先验证“游戏端口有流量/无流量”的诊断链路；真实游戏进程流量捕获尚未开始。
2026-06-10：已接入 ping 派生的 tunnel/P2P 观测，可先验证“能否互 ping、延迟、丢包”的诊断链路；真实加密隧道状态尚未接入。
2026-06-10：已接入本机 UDP 广播测试生成的 broadcast 包观测，可先验证“是否看见广播包”的诊断链路；真实广播转发链路尚未实现。
2026-06-11：桌面测试壳网络诊断后会把 `virtual_adapter`、`tunnel`、`p2p`、`broadcast`、`game_traffic` 摘要同步到房间详情，并给出下一步动作；真实游戏联机和真实流量采集仍未开始。
2026-06-11：已通过桌面壳参数链路和 CLI 冒烟测试验证：UDP loopback + UDP broadcast 写入同一包观测文件后，`network-observe` 可识别 `broadcast=seen` 和 `game_traffic=seen`；真实游戏进程流量捕获尚未开始。
2026-06-11：桌面测试壳的 UDP/TCP/广播测试按钮已接入“测试后自动网络诊断刷新”，输出区保留测试结果和诊断 JSON，右侧详情同步更新广播/游戏流量状态；真实游戏进程流量捕获尚未开始。
```

### M4：小范围测试

状态：未开始

计划：

- [ ] 邀请 5-20 名用户测试。
- [ ] 收集失败日志。
- [ ] 统计 P2P 成功率。
- [ ] 整理常见问题。

记录：

```text
2026-06-10：已明确 M4 用于验证真实网络成功率和失败案例，不提前扩大功能范围。
```

## 决策记录

### 决策 001

日期：2026-06-10

问题：第一版平台范围如何限定？

结论：按 Windows-only MVP 推进。

理由：Steam/PC LAN 游戏主要集中在 Windows，虚拟网卡、防火墙、网卡优先级也是当前产品成败的核心变量。

影响：暂不规划 macOS、Linux、Steam Deck 的实现；测试环境优先准备 Windows。

### 决策 002

日期：2026-06-10

问题：是否一开始自研完整二层虚拟 LAN？

结论：不从完整二层桥接开始，先验证三层虚拟网络、P2P 隧道、UDP 广播代理和 Direct IP 兜底。

理由：完整二层桥接会显著提高驱动、广播泛洪、稳定性和安全边界成本；功能设计文档也建议第一版不要追求完整真实二层局域网。

影响：M1/M3 重点验证 UDP 广播代理能否覆盖目标游戏，而不是默认转发整个家庭局域网。

### 决策 003

日期：2026-06-10

问题：是否接受轻量协调服务？

结论：正式产品默认接受轻量协调服务，但 PoC 阶段允许先用手动模式或本地配置验证网络路径。

理由：完全无后端对普通用户 NAT 场景失败率较高；轻量协调服务只承担房间创建、候选地址交换、NAT 打洞和短期保活，不承载游戏数据。

影响：后续架构需要预留 coordination endpoint、临时房间、一次性密钥材料和短期状态清理。

### 决策 004

日期：2026-06-10

问题：最终 Windows 客户端是否继续使用脚本运行时作为核心实现？

结论：不把脚本运行时作为最终网络核心；最终方向调整为 Windows 客户端 + Rust 原生核心。早期脚本原型已删除，后续不再作为实施基线。

理由：虚拟网卡、隧道、广播代理、NAT 诊断和防火墙检测属于性能敏感且接近系统边界的能力，Rust 更适合作为长期核心实现。

影响：新增 `native/` Rust workspace；后续需要安装 Rust 工具链，并优先让 `native/crates/lai-core` 通过编译和测试。

## 问题记录

### 问题 001

日期：2026-06-10

现象：当时仓库只有设计文档和进度文档，没有代码工程。

环境：`D:\work\code\LocalAreaInterconnection`

初步判断：项目当时处于规划阶段，下一步应先补项目基建或进入 M0 调研。

处理结果：已新增 `native/` Rust workspace、`windows-cli/` 测试壳、文档和测试；该问题已不再是当前阻塞。

状态：已解决

### 问题 002

日期：2026-06-10

现象：当前目录曾不是 Git 仓库。

环境：执行 `git status --short` 返回 `fatal: not a git repository`。

初步判断：缺少版本管理会影响后续断点推进和变更追踪。

处理结果：2026-06-11 已执行 `git init`，并补充 `.gitignore`；尚未创建首次提交。

状态：已解决

### 问题 003

日期：2026-06-10

现象：当前机器曾没有 Rust 工具链。

环境：执行 `cargo test` 返回 `cargo : The term 'cargo' is not recognized`。

初步判断：当时无法在当前环境编译或测试 `native/` Rust 工程。

处理结果：2026-06-11 已确认 `cargo 1.96.0` 和 `rustc 1.96.0` 可用；已在 `native/` 下执行 `cargo fmt` 和 `cargo test`，并修复编译问题，20 个核心测试通过。

状态：已解决

## 游戏兼容性记录

| 游戏 | Steam App ID | LAN 发现方式 | Direct IP | 当前结果 | 备注 |
|---|---:|---|---|---|---|
| 待填写 |  |  |  | 未测试 | 需要先确认第一批测试游戏 |

## 下一步

- [ ] 初始化 Git 仓库。
- [x] 安装/确认 Rust stable 工具链，并在 `native/` 下执行 `cargo test`。
- [ ] 开始 M0 调研，先整理 5-10 款候选 LAN 游戏。
- [ ] 记录现有工具对比表和兼容性表。
- [ ] 决定 Windows UI 壳：Tauri、原生 WinUI/WPF，或其他方案。
- [ ] 准备 M1 的两台 Windows 测试环境和虚拟网卡方案。
- [x] 将 `network_observation` 初步接入 Windows 网卡 `netsh` 观测文本。
- [x] 将 `network_observation` 初步接入 UDP/TCP 测试生成的 packet observation 文件。
- [x] 将 `network_observation` 初步接入 ping 派生的 tunnel/P2P 连通性观测。
- [x] 将 `network_observation` 初步接入 UDP 广播测试生成的 broadcast packet observation。
- [x] 在 Windows 测试壳中新增只读 `diagnostic-export`，用于导出 M1/M3 实验诊断包。
- [x] 修正 Windows 产物入口：`LocalAreaInterconnection.exe` 为可双击桌面程序，`LocalAreaInterconnection.Cli.exe` 为 CLI 后端。
- [x] 在桌面测试壳中新增复制邀请码、复制虚拟 IP、网络诊断和导出诊断包操作。
- [x] 在桌面测试壳中新增房间详情摘要，显示房间、成员/IP、连接状态、广播/游戏流量和下一步建议。
- [x] 在桌面测试壳中新增包观测文件输入，并串联 UDP/TCP/广播测试、网络诊断和诊断导出。
- [x] 在桌面测试壳中让 UDP/TCP/广播测试后自动刷新网络诊断和右侧房间详情。
- [x] 修复 `native/` Rust 编译问题，执行 `cargo fmt` 并通过 `cargo test`。
- [x] 为 Rust CLI 增加集成测试，固定关键 JSON 输出 schema。
- [x] 将 `diagnostic-export` 从 Windows C# 测试壳迁移到 Rust 原生核心边界。
- [x] 在 Rust core 新增本地房间成员生命周期模型，用于后续接入真实成员列表、连接路径和延迟。
- [x] 在 Rust core 新增 runtime observation 转换边界，用于未来接入真实隧道服务状态和真实广播/游戏包捕获摘要。
- [x] 创建首次 Git 提交。
- [ ] 将 `network_observation` 接入真实隧道服务实现和真实广播/游戏包捕获实现。
- [ ] 将桌面测试壳房间详情接入 Rust 房间生命周期模型或未来真实成员服务。
