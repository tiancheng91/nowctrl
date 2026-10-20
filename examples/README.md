# 设备端 client / server 二开示例

这里的 **client 是发出控制指令的 ESP32-C3**，**server 是接收指令的 ESP32-C3 Node**。
两端都使用 Rust `no_std`、Embassy 和 ESP-NOW，不需要路由器。
这些示例独立于原来的 P0 默认固件，也不连接 Python 管理页模拟器。

## 目录结构

```text
examples/
├── switch/
│   ├── client.rs       # 开关遥控端
│   └── server.rs       # 开关节点端
├── rc/
│   ├── client.rs       # 模玩遥控端
│   └── server.rs       # 模玩节点端
├── host/
│   └── loopback.rs     # 无板回环
├── common/
│   ├── mod.rs          # ESP32-C3 传输初始化
│   └── wire.rs         # 共用实验编解码
└── README.md
```

按业务分组，每组内区分 client/server；共用部分不复制。Cargo 显式指定源码路径，
因此目录调整不改变 `--example switch_client` 等命令和产物名称。

## 先看哪个示例

| 示例 | 作用 | 主要修改位置 |
| --- | --- | --- |
| `switch/client.rs` | 自动轮流发送 ON / OFF / TOGGLE / 500ms PULSE；等待 ACK，超时重试 | `COMMANDS`，替换成自己的按键/业务输入 |
| `switch/server.rs` | 解析指令，虚拟状态变化、去重、脉冲结束，回应用 ACK | `on_output_changed`，自己的状态变化回调 |
| `rc/client.rs` | 演示前进、快速前进、后退、左、右、停止；周期发送新帧 | `ACTIONS`，替换成自己的遥控输入 |
| `rc/server.rs` | 更新虚拟油门/转向；丢帧、重复帧不续期，300ms 无新帧归零 | `on_axes_changed`，自己的轴值回调 |
| `host/loopback.rs` | 电脑上直接跑同一套编解码和业务逻辑，模拟丢 ACK、重复包与 RC 断流 | 不需要 MAC 或设备，适合先改逻辑 |

**所有 server 回调目前只打印日志，不连接任何 GPIO、继电器、电机或舵机。**
传输使用明文单播和固定 MAC 过滤，仅适用于虚拟输出的实验。MAC 不是认证；
接入真实输出前，必须完成配对/加密、会话与重放保护、输出校准及独立安全停止。

## 1. 没有设备时先跑回环

在项目根目录执行（macOS Apple Silicon）：

```sh
cargo run --locked --example host_loopback --features host-examples --target aarch64-apple-darwin
cargo test --locked --lib --target aarch64-apple-darwin
```

Linux x86_64 把目标换成 `x86_64-unknown-linux-gnu`。
正常输出包含：

```text
switch server: output=true request=1 retry=false
switch server: output=true request=1 retry=true
switch client: application ACK accepted; duplicate toggle did not reexecute
RC server: Axes { throttle: 35, steering: 0 }
RC server: stopped at 300ms without a fresh update
PASS: host loopback only; no radio, GPIO or device authentication was tested
```

它是纯内存回环，不是两个网络进程，也不模拟 ESP32 外设；可以用于快速验证自己的业务逻辑。

## 2. 构建设备示例

```sh
cargo build --locked --release --features device-examples --example switch_client
cargo build --locked --release --features device-examples --example switch_server
cargo build --locked --release --features device-examples --example rc_client
cargo build --locked --release --features device-examples --example rc_server
```

四个独立 ELF 位于 `target/riscv32imc-unknown-none-elf/release/examples/`，文件名对应示例名，
不会覆盖默认的 `nowctrl` 固件。`device-examples` 显式启用这些实验固件，默认构建不包含它们。

## 3. 以后拿到板子时配置一对

1. 从各板的串口启动日志读取 **Station MAC**。可以先运行现有 P0 固件；
   示例也会先打印自己的 MAC，再检查 peer 配置。
2. 修改 `examples/common/mod.rs` 的 `CLIENT_MAC`、`SWITCH_SERVER_MAC` 或 `RC_SERVER_MAC`。
   必须填真实的非零单播 MAC，两端互相指向，不能填自己的 MAC。
   全零默认值会明确拒绝启动，不会自动广播控制指令。
3. 开关组合运行 `switch_client` + `switch_server`；模玩组合运行 `rc_client` + `rc_server`。
   一次先运行一对；四个文件不是一块板上同时运行的四个任务。
4. 通道固定为 RF 6。示例逻辑 Node ID 固定为开关 `7`、模玩 `8`，在 `common/wire.rs` 中定义，
   它们不是用户看到的逻辑 CH 编号，也不自动提供设备身份认证。
5. 明确对应板的串口和刷写方式后，分别刷写，例如：

```sh
espflash flash --chip esp32c3 --port <client-port> --monitor target/riscv32imc-unknown-none-elf/release/examples/switch_client
espflash flash --chip esp32c3 --port <server-port> --monitor target/riscv32imc-unknown-none-elf/release/examples/switch_server
```

这两条是后续实机步骤，本次未执行。示例不会假设 BOOT/LED/电机引脚。
client 是自动演示序列，并非已经绑定物理遥控按键。

## 二开时保留哪些部分

- `common/mod.rs`：芯片/堆/运行时初始化、固定 peer 和 ESP-NOW 传输；换通信方式时主要替换这里。
- `common/wire.rs`：显式小端序帧、目标 ID 和 lab RC 编解码；client/server 共用，避免两边字段不一致。
- 开关：`Pending` 负责 ACK 对应与最多三次发送，`VirtualNode` 负责执行状态和去重。
  Link TX 成功不等于业务成功。重复请求沿用同一个 request_id。
- 模玩：`RcState` 负责 latest-frame-wins 与超时归零；每次更新使用新 seq，不重发旧油门帧。
  LEFT/RIGHT 仅转向，不自动给油。具体电机混控、转向范围由之后的执行器适配决定。
- 业务回调：先修改日志/自己的纯逻辑，再用主机测试检查重复包、断流和错误目标。
  真实输出适配必须独立保证脉冲期限与失联停止，不应依赖一个可能阻塞的发送回调。
- 自定义帧：同步修改协议版本/类型定义、两端编解码和测试；不要重新解释已有类型编号。
  破坏兼容的修改需要新版本。根 `SPEC.md` 是正式协议依据。

## 实验边界

开关使用正式 v1 CONTROL/CONTROL_ACK 编解码。模玩使用 v1 envelope 的 RC_UPDATE，
但 payload 专门以 `LAB` 开头，后接一个动作码（STOP=0，FORWARD=1，BOOST_FORWARD=2，
REVERSE=3，LEFT=4，RIGHT=5）。它只是示例协议，**不定义正式 RC_UPDATE payload**，
也不能替代发送时间校验、认证会话和防重放。不要将这个格式直接当作生产协议发布。

示例没有自动配对、密钥、Flash 保存或重启会话协商。request_id/seq 在同一运行会话中严格递增，
不可回绕。实验时单边重启会导致旧 ID 被另一端拒绝，应同时重启这一对；正式产品必须通过认证
会话重建解决，不能收到低序号就自行清空去重状态。

接收循环每 10ms 回到时间推进，驱动虚拟脉冲结束与 RC 看门狗。底层发送回调最多等 50ms；
当前 esp-radio 不允许取消后直接复用 sender，否则可能错认后续发送状态，因此回调超时会
**停止该示例**，不会继续复用。这个故障策略仅适用于无实际输出的实验，不能直接用于真实负载。

验证记录见 `docs/validation/device-examples.md`：四个设备 ELF 已编译，电脑回环与 28 项原生测试通过，
实际 ESP-NOW 收发、时序和 GPIO 没有验证；现有 ESP-EMU RTOS 启动阻塞也未绕过。
