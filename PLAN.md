# NowCtrl — Implementation Plan

> Version: 0.2 (2026-10-20)
> Canonical design: [SPEC.md](SPEC.md)
> Status: **P0 in progress, not accepted**. This document consolidates the former `espnow-remote` plans and subsequent ESP-EMU/RC decisions.

## Principles

1. Rust `no_std` + Embassy + esp-hal + esp-radio only for device firmware; no requirement to write ESP-IDF C.
2. Initial hardware: ESP32-C3 Mini board(s), BOOT and LED only; no attached actuators.
3. Distinguish **code written** from **compiled**, **emulator-tested**, and **real-board verified**. Only checked tasks have reliable evidence.
4. Keep firmware independent of ESP-EMU. Emulator tests supplement but do not replace physical radio tests.
5. Implement one phase at a time, keep a reproducible log and update this document.
6. Keep RC continuous control and switch actions as separate protocol semantics, even if RC implementation comes later.

## Current baseline (from earlier conversation)

- [x] Project name selected: `nowctrl`; public GitHub repository exists at `tiancheng91/nowctrl`.
- [x] Historical product/technical docs and an initial ESP32-C3 Rust firmware scaffold were drafted/submitted in earlier steps.
- [x] Consolidated latest system spec and executable roadmap into root `SPEC.md` and `PLAN.md` (this delivery).
- [ ] Confirm local checkout is synchronized with current repository and new documents.
- [x] Confirm the existing scaffold **builds** with coherent pinned esp-rs dependencies and generates a lockfile (both roles; see `docs/validation/p0.md`).
- [x] Confirm either Controller or Node firmware has **booted** on ESP-EMU or hardware (both emulator application startup banners observed; RTOS/timer gate remains blocked).
- [ ] Confirm ESP-NOW actually transmits and receives using two boards.

Existing P0 prototype in the earlier repository broadcasts role-specific messages; it is **not** an authenticated controller, doesn't drive a real output, and does not implement HTTP or advanced pairing. Prior file creation is not functional acceptance.

---

## P0 · Firmware foundation and ESP-EMU feasibility (NOW)

### P0.1 Environment and reproducibility
- [x] Check actual local tree, compiler/toolchain availability, firmware build feature flags (Rust 1.99.1; both invalid role combinations rejected).
- [x] Select matching `esp-hal`, `esp-radio`, `esp-rtos`, bootloader and Embassy revisions from one verified official example; remove dependency skew and include `Cargo.lock` in delivery (no Git commit requested).
- [x] Build both firmware roles independently (`controller` default; `node` with default disabled). Document the precise commands.
- [ ] Verify ESP32-C3 board profile(s): BOOT pin, LED type/pin, serial port and flashing mode.

**Gate:** Both feature variants compile; record the toolchain and `Cargo.lock` versions. No assumption that existing source compiles.

### P0.2 ESP-EMU integration
- [x] Install/pin `esp-emu` on macOS Apple Silicon, document version and path (existing 0.49.0 installation verified; scripts enforce version).
- [x] Add `sim/esp-emu/README.md`, build/launch scripts; avoid brittle hard-coded local paths.
- [x] Generate valid ESP32-C3 flash binary/merged image from Rust build (bootloader + partition table + app as required) and verify offsets (both roles: 0x0 / 0x8000 / 0x10000).
- [x] Boot the Controller firmware in ESP-EMU and capture startup banner (USB-JTAG console; see logs).
- [ ] Validate Embassy timer tick repeats and GPIO output where the emulator supports it.
- [x] Attempt Node image startup, record pass/fail and incompatibilities (application starts; RTOS startup does not return in 20 seconds).
- [ ] Probe ESP-NOW initialization; clearly record whether emulated TX/RX and multi-instance delivery work, without assuming they do.
- [ ] Probe SoftAP/HTTP separately and any actual coexistence outcome, if testable.

**Gate:** At least one real Rust firmware image boots in ESP-EMU and runs an async timer; unsupported peripheral details are documented with logs. A non-working ESP-NOW emulator does **not** block proceeding to physical tests.

### P0.3 Two physical devices and coexistence
- [ ] Flash distinct role images onto two real ESP32-C3 Mini boards.
- [ ] Verify fixed RF channel 6, broadcast discovery and bidirectional unicast exchange; log MAC + monotonic sequence.
- [ ] Verify power-cycle/restart recovery (temporary fixed peer is acceptable at P0).
- [ ] Independently launch SoftAP and minimal HTTP on Controller and connect from a phone.
- [ ] Test ESP-NOW while SoftAP is running using the *same locked esp-radio version*; measure whether RX/TX and HTTP remain operational.
- [ ] If coexistence fails, implement/document a safe, explicit setup-mode transition, with control disabled in configuration mode.
- [x] Save evidence under `docs/validation/p0.md`, separating host/emulator/hardware results (hardware remains untested).

**P0 exit criteria:** both roles build; emulator firmware boot + Embassy tick verified or blocker documented; **actual** dual-board ESP-NOW communications verified; phone HTTP connectivity verified; AP/ESP-NOW coexistence concluded or a tested fallback exists. Do not close P0 on emulator-only results.

### P0 outputs

- Role-specific build artifacts; reproducible build/flash commands.
- Pinning/lockfile of dependencies and ESP-EMU release.
- `sim/esp-emu/` scripts and README.
- `docs/validation/p0.md` with logs, failures, versions, and explicit limits.
- Updated project `README.md`, `SPEC.md` and this `PLAN.md`.

---

## P1 · First switch-control loop

Software-only work is proceeding at the user’s request while physical P0
validation is deferred. P0 remains unaccepted; native results do not satisfy
the two-board gate.

- [x] Extract or create host-testable packet encode/decode and payload validation, versioning and bounds checks (v1 envelope + switch CONTROL/ACK; 11 native tests; radio integration and other payload schemas remain pending).
- [x] Host-testable debounce and release-resolved gesture priorities (30ms / 300ms / 3s / 8s / 15s defaults).
- [ ] Bind button events and LED output to a verified board profile; physical reset confirmation remains pending.
- [x] Native virtual Node switch transitions and pulse deadlines; host tests cover wrong-node rejection.
- [ ] Integrate Controller selection and Node virtual commands into firmware tasks/radio.
- [x] Native ACK correlation, bounded retry/backoff and latest-request cache; duplicates cannot repeat TOGGLE/PULSE (stop-and-wait, monotonic IDs per authenticated session).
- [ ] Integrate application ACK/retry/cache with validated ESP-NOW transport identity and sessions.
- [x] Host-testable finite LED level patterns for CH N, success, failure and offline; GPIO driver/state scheduling remains pending.
- [x] Native unit tests and deterministic 100-command loss/duplicate/wrong-node test.
- [ ] Repeatable 100-command two-board test (deferred until hardware is available).

**Gate:** No wrong-node or repeated execution across 100 control attempts with controlled retries; boot recoverable. P1 is still developmental and no actual GPIO load is attached.

## P2 · Multi-node management and persistence

- [ ] Discovery/temporary physical pairing windows; stable Node IDs and add-order channel numbers.
- [ ] Versioned atomic Flash persistence of role, pair info, order/name, settings and selected node.
- [ ] Manage up to eight product-configured Nodes (validate radio encrypted-peer limitations).
- [ ] Reorder channels without altering identities; delete/disconnect with guarded confirmation.
- [ ] Online/offline metrics and reconnect after power-cycle.

**Gate:** Three real Nodes pair, reorder and reboot without routing wrong commands. Production secure enrollment may require P4 hardening before shipping.

## P3 · Mobile Web setup

- [x] Offline management assets following root `DESIGN.md`, plus loopback-only simulation API for pairing preview, names, enable/disable, reorder, guarded remove and virtual switch actions.
- [x] Host file persistence with atomic replace/rollback tests; assets exposed through Rust `include_str!` for a future firmware HTTP adapter. This is not firmware flash persistence.

- [ ] Protected, timed Controller SoftAP; DHCP/TCP stack and lightweight HTTP service.
- [ ] Device list, pairing, names, enabled flags, logical order and system information endpoints.
- [ ] Mobile UI embedded in flash; atomic reorder, validation and authenticated mutations.
- [ ] Preserve radio control in parallel if validated, otherwise provide explicit safe config-mode switch.

**Gate:** Phone manages three real Nodes offline, with persisted configuration and no unintentional actuation.

## P4 · Security, reliability and release gating

- [ ] Production-grade provisioning/key exchange, encrypted unicast and unique device credentials.
- [ ] Anti-replay/session reset design; fuzz malformed packets and storage recovery.
- [ ] Link-quality, offline and application ACK metrics; radio interference stress tests.
- [ ] Verification of safe defaults, reject unpaired commands, secure AP and reset confirmation.

**Gate:** 500 control operations with zero wrong-device or duplicate executions, malformed/unauthorized command rejection, documented security boundaries.

## P5 · Low power and battery policy

- [ ] Controller button wake and LED/AP-off inactivity policies.
- [ ] Node always-listening vs light sleep/radio power-save vs scheduled wake trials.
- [ ] Measure **whole-board** current and response latency; prove wake pin suitability.
- [ ] Document hard limitation: ESP32-C3 deep sleeping radio cannot accept arbitrary incoming ESP-NOW messages.

**Gate:** documented current, wake latency and failure rates on real boards; default behavior does not silently drop responsiveness.

## P6 · GPIO actions and RC extensions

- [x] Software-only device presets: switch, RC basic actions, and unknown-device read-only GPIO fallback; API rejects unsupported output operations.
- [x] Native RC action mapping/receiver watchdog plus held-action management preview with STOP on release/blur/node change and a simulated 300ms watchdog. Actual RC radio protocol/freshness/actuator bindings remain pending.

- [ ] Safe GPIO output abstraction for ON/OFF/TOGGLE/PULSE; powered actuator integration after bench testing.
- [ ] Continuous RC inputs and outputs: joystick/axis, steering/throttle/PWM, latest-frame-wins, priority, deadlines.
- [ ] Mandatory neutral/stop watchdog on lost link, stale control, reset and invalid commands.
- [ ] Optimize active RC latency and jitter; disable low duty-cycle receive during RC control.
- [ ] Optional OTA, extra boards and config import/export after core stability.

**Gate:** switch output fail-safe tested; RC failsafe stops/neutralizes correctly on disconnection or freeze before connecting safety-critical loads.

---

## Latest execution evidence (2026-10-20)

Both roles link with Rust 1.99.1 and the pinned official-example revision. Merged
4MB images and bounded emulator scripts are available. ESP-EMU 0.49.0 reaches
application/HAL startup on both roles with `--console usb-jtag`, then stops
progressing during `esp_rtos::start`; no Embassy tick or radio initialization
was observed. P0.2 timer acceptance remains blocked, and P0.3 remains pending.
See `docs/validation/p0.md` for logs and reproduction commands.

Software-only continuation: the v1 protocol library now passes 11 native tests,
Clippy and both firmware builds. Native checks run without ESP hardware drivers;
a GitHub Actions workflow covers native tests and the firmware role matrix
(remote CI not yet run). The subsequent software continuation adds native virtual
state, ACK/retry/dedup, button/LED timing and the DESIGN.md management preview.
28 Rust tests and 12 management API tests pass; radio/task/HTTP/Flash integration
remains pending. See `docs/validation/software.md`.

## Immediate next action for a local coding agent

1. Read the new root `SPEC.md` and `PLAN.md` first, plus existing Cargo/source and `docs/PRODUCT.md`.
2. Reproduce the passing locked dual-role builds and verify the actual board profile.
3. Investigate the recorded ESP-EMU RTOS/software-interrupt startup blocker without modifying production architecture to fit the emulator. A documented emulator blocker does not prevent physical tests.
4. Continue firmware task/radio integration of the tested control/input core and implement directory/flash and protected embedded HTTP adapters. Keep the Python management preview separate from actual firmware capabilities. Do not mark hardware gates accepted from native tests.

## Device-preset continuation (2026-10-20)

At the user's request, defined switch/RC profiles remain operable from the Web;
undefined types display GPIO telemetry read-only. Software preview and native
logic now support basic RC actions separately from switch ACK/retry semantics.
Legacy host records without a profile load as read-only unknown devices. See
`docs/validation/presets.md` for checks and limits. No hardware acceptance is
advanced by this change.

## Device customization examples (2026-10-20)

- [x] Independent ESP32-C3 switch client/server and RC client/server examples with shared radio/codec helpers and clear business callbacks.
- [x] Native host loopback plus example-wire tests; 28 native tests pass.
- [x] All four example release ELFs link; native/example Clippy passes; CI covers example builds and native loopback (remote run pending).
- [ ] Real-device example boot/radio exchange and physical adapters (deferred; lab examples use virtual output and no pairing/encryption).

See `examples/README.md` and `docs/validation/device-examples.md` for commands,
customization points and the lab-only RC schema boundary.

## Key risks

- `esp-radio` API and SoftAP concurrent operation change across versions.
- ESP-EMU may emulate Wi-Fi AP but not ESP-NOW multi-instance radio frames, and may not boot a given esp-hal image without compatible flash layout.
- ESP32-C3 board BOOT GPIO strapping and LED part vary by manufacturer.
- Dev-board deep-sleep power may differ materially from chip datasheets.
- RC safety semantics cannot be inherited from switch retries or sleep strategies.
