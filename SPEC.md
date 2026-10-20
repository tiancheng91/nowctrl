# NowCtrl — Technical Specification

> Version: 0.2 (2026-10-20) · Status: P0 dual-role builds verified; emulator timer and hardware pending  
> Repo: https://github.com/tiancheng91/nowctrl  
> Product: Lightweight local wireless control for switches, RC models and DIY devices; built with Rust and Embassy.

## 1. Product definition and boundaries

NowCtrl is an open-source **one-controller-to-many-nodes** control system using ESP32-C3 Mini boards and ESP-NOW. A controller hosts a temporary Wi-Fi SoftAP + HTTP management UI; nodes communicate by ESP-NOW and **do not associate with the AP or obtain IP addresses**. Controller and Node roles share the same Rust modules, with distinct build features/binaries.

Initial hardware is limited to the boards' built-in BOOT buttons and status LEDs; GPIO actuators, joysticks, servos, MOSFETs and relays are future peripherals. No router, cloud or Internet is required for runtime control.

Two product modes must remain distinct:

- **Switch mode**: discrete ON/OFF/TOGGLE/PULSE, application acknowledgment, deduplication and bounded retries.
- **RC mode**: latest-value-wins axes/throttle/steering/servo commands, periodic updates, sequence/freshness checks, watchdog and **fail-safe neutral or output-off on loss of signal**. Never retry stale position/throttle frames. Do not depend on slow periodic sleep/listening during active RC control. This mode is planned, not part of the first prototype.

## 2. Requirements

| ID | Requirement | Priority |
| --- | --- | --- |
| FR-01 | One Controller with 1–8 Nodes (initial target; validate encrypted peer limit) | Must |
| FR-02 | Automatic discovery; physical-confirmation time-limited pairing; persistence across reboot | Must |
| FR-03 | Stable Node ID/MAC; assign CH1..CHN by addition order, reorder via Web without changing identity | Must |
| FR-04 | BOOT single click next channel, double click trigger; 3s pairing, 8s Web, 15s guarded reset (tentative) | Must |
| FR-05 | Blink N pulses for CH N only after selection; other distinguishable patterns; off when idle | Must |
| FR-06 | Protected temporary Controller SoftAP and local HTTP configuration page | Must |
| FR-07 | Virtual outputs initially; real GPIO commands in a later phase | Must |
| FR-08 | Switch ACK, timeout, retransmission, deduplication and negative acknowledgment | Must |
| FR-09 | Battery-oriented power management subject to responsiveness and hardware measurements | Should |
| FR-10 | RC continuous controls, latency budget, dead-man/failsafe and future input peripherals | Later |
| FR-11 | Fully automated native Rust tests + ESP-EMU firmware test path | Must |

## 3. Architectural decisions

- MCU: ESP32-C3 Mini, RISC-V, built-in button and LED pinout determined by **board profile**, not hardcoded until board verified. BOOT GPIO/bootstrapping and WS2812 vs simple LED require checks.
- Language/runtime: Rust `no_std`, `esp-hal`, `esp-radio` (ESP-NOW/Wi-Fi), `esp-rtos` + Embassy executor/time; use matching compatible versions or pinned official-example revisions and include `Cargo.lock`.
- TCP/IP: `embassy-net`; HTTP: `picoserve` or equivalent after AP proof, embedded HTML/CSS/JS.
- Persistence: flash-backed versioned, checksummed configuration with power-loss-safe write/rollback; storage crate to be selected by real flash tests. Avoid claiming ESP-IDF NVS compatibility until confirmed.
- Radio: initial fixed **Wi-Fi RF channel 6**. Logical user channels (CH1–CH8) are unrelated to 802.11 RF channel; ESP-NOW uses same RF channel and can share a PHY with AP subject to driver support.
- SoftAP: disabled normally; enter configuration on long press, password-protected, idle timeout. Test simultaneous ESP-NOW+SoftAP on the selected `esp-radio` version. Fallback is exclusive config mode, explicitly unavailable for control while configuring.
- Components: pure Rust protocol/core, hardware adapters, controller/node firmware, optional desktop network simulator and external ESP-EMU launcher.

### Logical diagram

```text
Phone browser --Wi-Fi SoftAP/HTTP (configuration only)--> Controller (ESP32-C3)
                                                       |  BOOT / LED / flash
                                                       |  ESP-NOW RF ch6
                               +-----------------------+-----------------------+
                               |                       |                       |
                           Node A                  Node B                  Node C
                      virtual GPIO / ACK      virtual GPIO / ACK      virtual GPIO / ACK
```

## 4. Device state and identity

Controller stores `controller_id`, paired node records, selected `node_id`, ordered array of `node_id`, AP credentials and power policy. Each Node stores `node_id`, Controller MAC/identity, RF channel, secrets and action policy. The Node ID is stable and independent of UI order. Online status derives from recent authenticated HELLO/ACK and is never blindly restored from flash.

Maximum eight initial Nodes is a **product target**, not a promise of eight concurrently encrypted hardware peers; confirm actual chip/driver limits.

Controller states: BOOT → NORMAL/ACTIVE → IDLE → LOW_POWER; on user request CONFIG or PAIRING; guarded FACTORY_RESET. Node states: UNPAIRED → PAIRING → PAIRED_LISTENING → IDLE/POWER_SAVE as allowed. A radio-disconnected node cannot claim to be online.

## 5. Button and LED interaction

| Controller action | Intended behavior |
| --- | --- |
| Single short press | Select next enabled Node, wrap at end |
| Double press | Send current Node's selected switch action (future RC input needs different UI) |
| Hold ≥ 3 seconds | Physical pairing authorization window (~60s) |
| Hold ≥ 8 seconds | Enable/disable temporary Web config |
| Hold ≥ 15 seconds | Guarded factory reset with distinct confirmation sequence |

Resolve overlapping long-press gestures on **button release**, or explicitly consume a shorter gesture when a longer threshold is reached; never run pairing, Web, and reset accidentally on the same hold. Debounce and multi-click windows must be tunable.

After channel selection LED blinks N times (approx. on 120 ms/off 180 ms, repeat two groups and turn off), with distinct patterns for pairing, Web, offline, successful trigger and failure. Limit LED activity for battery life. Node indicates pairing/result. Actual LED driving depends on board's LED type.

## 6. Protocol and transport

Use explicitly serialized **little-endian** frames (do not transmute Rust structs), versioned and length-checked, with compact payloads (target ≤ 200 bytes to support legacy ESP-NOW payload constraints).

Proposed header: `version:u8`, `type:u8`, `flags:u16`, `node_id:u32`, `seq:u32`, `request_id:u32`, `payload_len:u16`; source/destination resolved through transport identity and authenticated context. Frame types: `DISCOVER`, `PAIR_REQ`, `PAIR_CHALLENGE`, `PAIR_CONFIRM`, `PAIR_ACCEPT`, `HELLO`, `HEARTBEAT`, `CONTROL`, `CONTROL_ACK`, `CONFIG_SET`, `CONFIG_ACK`, `STATUS_REQ`, `STATUS_RESP`, `RC_UPDATE` (later).

- **Switch**: Controller issues unique `request_id`; Node deduplicates and returns cached ACK/result; retries must never repeat TOGGLE/PULSE. Link-level TX status is not business success. Retries bounded and backoff tunable.
- **P1 software baseline**: virtual switch/ACK logic uses stop-and-wait, at most three sends (200ms, 400ms retry intervals, then an 800ms final ACK window). Within one externally authenticated session, request IDs are nonzero and strictly increasing, without wrap. The latest request caches its result; identical latest retries return that result, altered retries and older IDs are rejected. This core is native-tested but not yet called by P0 firmware. Secure session reset and replay protection still require P4 integration.
- **RC**: sequence + sender timestamp/session ID (where possible); freshness window; latest-value-wins; no delivery-ACK dependency for each control frame; fail-safe after timeout based on measured latency and actuator risk. Define neutral outputs before enabling real GPIO/PWM.
- **Heartbeats**: infer online/offline with timeout, avoid flooding and separate configuration traffic from control priority.
- **Pairing**: discover with broadcast only in physical time window. Broadcasting alone never establishes trust. Development-only pre-shared secrets may be used for early test; release pairing must use device-unique authenticated provisioning/key establishment and encrypted unicast, replay-resistant across resets.

### 6.1 P1 wire baseline (native-tested, not yet used by P0 radio)

The single-crate `no_std` library now defines explicit v1 encoding. P0 firmware
continues to send its original discovery strings; it does not accept or execute
these control packets yet. Frame decoding is validation, **not authentication**.

| Offset | Field | Encoding |
| --- | --- | --- |
| 0 | version | u8, 1 |
| 1 | type | u8, assignments below |
| 2 | flags | u16 LE, reserved; must be zero in v1 |
| 4 | node_id | u32 LE |
| 8 | seq | u32 LE |
| 12 | request_id | u32 LE |
| 16 | payload_len | u16 LE |
| 18 | payload | exact declared length |

Total length is 18–200 bytes, leaving at most 182 payload bytes. Both truncation
and trailing bytes are rejected. Type assignments follow the order above:
DISCOVER=1, PAIR_REQ=2, PAIR_CHALLENGE=3, PAIR_CONFIRM=4, PAIR_ACCEPT=5,
HELLO=6, HEARTBEAT=7, CONTROL=8, CONTROL_ACK=9, CONFIG_SET=10, CONFIG_ACK=11,
STATUS_REQ=12, STATUS_RESP=13, RC_UPDATE=14. Other type values are rejected.

CONTROL payloads: OFF=`01`, ON=`02`, TOGGLE=`03`, PULSE=`04` followed by a
nonzero u32 LE duration in milliseconds. The first three have exactly one byte;
PULSE has exactly five. The action policy must impose any product-specific pulse
limit before execution; codec acceptance does not authorize a duration or output.
CONTROL_ACK is exactly one byte: 0=Applied, 1=Rejected. This is the initial generic
negative acknowledgment; detailed rejection codes are not assigned yet.
Both CONTROL and CONTROL_ACK require nonzero `node_id` and `request_id`.

Other assigned message types currently receive envelope/length checks only;
their payload schemas, semantic handlers and RC safety checks remain unimplemented.
An assigned type does not mean its feature is available. ACK correlation,
authenticated sender checks, retry bounds and deduplication must be implemented
before connecting this codec to virtual control dispatch. No live control path
is added by this native-test milestone.

## 7. Security and safe actuation

MAC is not authentication. ESP-NOW encrypted unicast after secure pairing; do not transmit real actuation commands by plaintext broadcast. Confirm ESP-NOW encryption peer limit and PMK/LMK setup on selected driver. Provide replay protection with session nonce/epoch and sequence; persist relevant counters safely or rekey on restart. AP uses unique WPA2 password and authenticated modification APIs; credentials must not use hardcoded public defaults. Local physical action required for destructive changes.

All future hardware outputs default to safe state on boot, lost session, network timeout and watchdog reset. Motor/RC controls need explicit failsafe, independently of radio ACK. First prototype controls **virtual output state only**, no connected loads.

## 8. HTTP management scope (later P3)

Phone joins temporary SSID `NowCtrl-XXXX`, normally at `192.168.4.1`. Planned REST endpoints: `GET /api/v1/status`, `GET /api/v1/nodes`, `POST /api/v1/pairing/start`, `PUT /api/v1/nodes/order`, `PATCH /api/v1/nodes/{id}`, `DELETE /api/v1/nodes/{id}`, `POST /api/v1/nodes/{id}/control`. JSON response `{ok,data,error}`. Validate inputs, authorize edits, send storage commands to background queue; ensure atomic reorder.

UI features: node discovery/pairing, rename/order, online indicator, virtual control preview, action profile and system info. Hosted locally; no external assets required.

### 8.1 Management software preview

The offline management assets follow root `DESIGN.md` (dark industrial panels,
orange actions, cyan communication details, fine borders and local font fallbacks).
A loopback-only Python service exercises the API/UI with virtual nodes while
hardware is unavailable. It explicitly identifies simulated status and ACKs,
uses a per-process mutation token and Host/Origin checks, and persists only the
host preview directory/order/name/enabled configuration using atomic file replace.
It is not firmware Flash storage, secure physical pairing or a radio bridge.

Preview-only `/api/v1/demo/nodes` and `/api/v1/demo/events` endpoints must not be
exposed by a production controller. The assets are available as Rust `include_str!`
constants but are not served by the current P0 firmware. The actual protected
SoftAP/HTTP task and firmware adapter remain pending. See `web/README.md`.

### 8.2 Device presets and read-only GPIO fallback

Primary usage is switch control and model/toy RC. Devices declare `device_type`:
`switch`, `rc_basic`, or `unknown`. Missing/unrecognized device types fall back to
read-only GPIO display; names never determine capabilities. Defined profiles may
expose actions in the Web page. Unknown devices display HIGH/LOW/unknown telemetry
and reject output operations at the API/dispatch boundary, not only by hiding UI
buttons. Metadata (name/order/enabled) remains manageable. There is no generic
Web GPIO write operation. Actual GPIO inventory must come from the device profile,
not guessed board pins; missing telemetry must not be shown as LOW.

The initial RC basic action set is STOP, FORWARD, BOOST_FORWARD, REVERSE, LEFT and
RIGHT. The software preview maps throttle to 0/+35/+65/-30 percent; LEFT/RIGHT
map steering to -100/+100 percent and throttle zero. These logical values are
not calibrated actuator outputs, and no differential wheel mixing is implied.
Web actions are held, periodically refreshed and stopped on release/cancel,
focus/visibility loss, page exit, node change or disable. RC updates use increasing
sequence numbers and latest-frame-wins, without switch retries or application ACK
dependence. The preview/core watchdog is 300ms; physical thresholds, sender-frame
freshness checks, authenticated sessions and actuator neutral calibration still
require implementation/measurement before real RC outputs can be enabled.

The preview adds `/api/v1/nodes/{id}/rc` and profile/RC/GPIO data to node responses;
this is a software adapter contract, not a completed firmware RC_UPDATE wire schema.
Demo storage v2 adds device_type. Legacy v1 records without it load as unknown,
preserve metadata, and write v2 only on a successful later configuration mutation.
Device profiles cannot be changed via the node PATCH endpoint; mock nodes may
explicitly declare their simulated type at creation.

## 9. Async architecture and test seams

Tasks: `radio_rx`, `radio_tx`, `button`, `led`, `controller`, `node`, `storage`, `power`; `web` only in Controller. Use bounded Embassy channels/signals, no blocking Flash operations or expensive work in radio callbacks. Keep these concerns separate:

- `nowctrl-core`: no_std messages, serialization, request de-dupe, ordering, pairing/connection state, RC watchdog logic; host `cargo test`.
- `nowctrl-firmware`: esp-hal drivers, esp-radio transport, Embassy tasks, role features.
- `nowctrl-sim` (optional, later): host fake radio for packet loss, latency, drop/reorder/duplication tests; **not** a silicon simulator.
- `sim/esp-emu`: external emulator wrapper to boot actual ESP32-C3 images and probe supported peripherals; **not** a production firmware dependency.

Avoid premature workspace refactor during P0: begin with existing single-crate `src/main.rs` and extract modules once compiling.

### 9.1 Device customization examples

`examples/` provides independent ESP32-C3 switch_client/switch_server and
rc_client/rc_server binaries, explicitly enabled with `device-examples`.
They share pinned HAL/radio setup and lab packet helpers; per-application
command/input lists and output/axis callbacks remain in the example entrypoints.
A native host_loopback (feature `host-examples`) exercises those same helpers
without a board. Both forms reuse the no_std core, not the Python preview model.

These examples demonstrate virtual output only with fixed, explicitly configured
unicast peers on RF 6. MAC filtering is not authentication. They have no secure
enrollment, persistence, session recovery or physical output adapters. RC example
payloads are scoped with a `LAB` prefix and do not establish the production
RC_UPDATE schema. Default P0 firmware is unchanged. See `examples/README.md`.

## 10. ESP-EMU integration plan and limits

Reference: https://github.com/espressif/esp-emulator . Emulator advertised ESP32-C3 CPU/flash/GPIO/timers/UART and Wi-Fi/SoftAP behavior; **ESP-NOW multi-instance delivery is not confirmed**. Evaluate actual esp-hal boot compatibility rather than assuming an ESP-IDF example demonstrates Rust Embassy compatibility.

1. Pin a working ESP-EMU release; record host macOS Apple Silicon commands and version.
2. Build the `riscv32imc-unknown-none-elf` firmware with consistent esp-rs dependencies; lock `Cargo.lock`.
3. Produce a valid **merged flash image** with bootloader/partition/app offsets, if required; never rename ELF into `.bin`.
4. Run `esp-emu --chip esp32c3 --firmware <merged.bin>`; verify UART boot banner and Embassy periodic tick.
5. Test GPIO/LED if supported by the chosen virtual board profile; diagnose difference between GPIO model and external WS2812.
6. Independently probe ESP-NOW initialization/transmit and SoftAP/HTTP + possible coexistence. Record supported/unsupported/unknown and emulator logs; **do not equate emulated Wi-Fi SoftAP with ESP-NOW support**.
7. Repeat radio exchange on **two physical ESP32-C3 boards**. Simulator cannot certify range, packet loss under RF interference, or battery current.

Current P0 baseline (2026-10-20): Rust 1.99.1, esp-rs revision
`c21d5741a8fdecb9e01b196e55b7c7859360bf1b`, espflash 4.4.0 and ESP-EMU 0.49.0.
Both roles compile and enter the application on the emulator, but RTOS startup
has not completed; no timer/radio acceptance is claimed. Native runs explicitly
select the USB-JTAG console to observe esp-println output. Detailed evidence is
in `docs/validation/p0.md`.

ESP-EMU hosted browser build does not provide networking by default; prefer native CLI for network tests. The emulator README emphasizes ESP-IDF merged images; treat Rust image boot as a real P0 experiment. Do not bend production architecture for emulator support.

## 11. Power policy

ESP32-C3 Deep-sleep disables its radio; Node cannot be remotely awakened by arbitrary ESP-NOW packets while fully asleep. Controller may sleep with suitable physical wake GPIO, subject to actual BOOT pin/strapping validation. Node choices: continuously listening for responsive controls, light-sleep/power-save with validated radio support, or scheduled wake windows with bounded delay. **Active RC requires responsive receive/failsafe, so do not enable periodic deep-sleep during control.**

USB-powered dev-board current includes regulator/USB/UART/LED load; evaluate *whole-board* current on actual boards. Initial default: reliability over battery life; AP off outside config; LED off when idle.

## 12. Repo layout (target; do not demand all directories on day one)

```text
nowctrl/
├── README.md
├── SPEC.md                 # canonical system and simulation specification
├── PLAN.md                 # stages, objective gates and honest test status
├── Cargo.toml
├── Cargo.lock              # after dependency resolution
├── rust-toolchain.toml
├── src/main.rs             # initial P0 Controller/Node firmware
├── crates/                 # later: core/protocol when justified
├── sim/esp-emu/            # version-pinned launch/build scripts and evidence
├── docs/
│   ├── PRODUCT.md          # product-level requirements
│   └── validation/         # test results and hardware notes
└── tests/
```

`docs/DESIGN.md` and the original `espnow-remote/SPEC.md` are historical inputs; the root `SPEC.md` is authoritative going forward. Maintain a single `PLAN.md` at project root.

## 13. Verification and non-goals

**Evidence required:** successful Cargo builds for both features, emulator boot logs, timer and GPIO results if supported, actual ESP-NOW two-board logs with device IDs/versions, SoftAP test, and a clear supported/unsupported conclusion for coexistence. Tests must be classified: host logic, emulator silicon/peripheral, real radio/hardware. Never mark work complete solely because files exist.

Not in P0: actual relays/MOS/motor/servo outputs; production secure pairing; feature-complete Web UI; RC control; long-term deep-sleep optimization; OTA. These remain planned milestones.

## 14. References

- Original project design: `espnow-remote-design.zip` (historical)
- ESP-RS official Embassy ESP-NOW duplex sample: https://github.com/esp-rs/esp-hal/tree/main/examples/esp-now/embassy_esp_now_duplex
- ESP-EMU documentation: https://github.com/espressif/esp-emulator
- Existing repository: https://github.com/tiancheng91/nowctrl
