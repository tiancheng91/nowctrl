# Device client/server examples — 2026-10-20

## Delivery

Four independent `no_std` ESP32-C3 binaries are available under `examples/`:
`switch_client`, `switch_server`, `rc_client`, `rc_server`. They share transport
setup and packet helpers but keep the business sequence/callback in each small
entrypoint. Default P0 firmware and the management preview are unchanged.
A fifth, native `host_loopback` example exercises the identical packet helpers
without radio or a board.

The device binaries require `device-examples`; the native binary requires
`host-examples`. This keeps firmware demos out of normal native/default builds.
Zero peer MACs intentionally refuse runtime startup after printing the local
station MAC; users must configure the correct unicast peer explicitly. No
example drives GPIO, PWM, motors, servos, relays or an actual LED.

## Actual checks

Commands were run from the repository root:

```sh
cargo test --locked --lib --target aarch64-apple-darwin
cargo run --locked --example host_loopback --features host-examples --target aarch64-apple-darwin
cargo build --locked --release --features device-examples --example switch_client --example switch_server --example rc_client --example rc_server
cargo clippy --locked --lib --target aarch64-apple-darwin -- -D warnings
cargo clippy --locked --example host_loopback --features host-examples --target aarch64-apple-darwin -- -D warnings
cargo clippy --locked --release --features device-examples --example switch_client --example switch_server --example rc_client --example rc_server -- -D warnings
cargo fmt --check
```

- [28 native tests pass](logs/device-example-tests.log), including three tests of
  the exact example helpers: switch retry/ACK, all RC actions and watchdog, and
  malformed/foreign lab-RC payload rejection.
- [Host loopback passes](logs/device-example-loopback.log): a dropped first ACK
  produces a retry without repeated TOGGLE; a forward update becomes neutral
  after 300ms with no fresh frame.
- [Four release ELFs link](logs/device-example-builds.log), under
  `target/riscv32imc-unknown-none-elf/release/examples/`.
- [Clippy passes](logs/device-example-clippy.log), native and all device examples.
- CI configuration includes native loopback and firmware example build/lint;
  the remote workflow has not been triggered, and nothing was committed/pushed.

The first native example test harness was an integration test. Cargo then tried
building the default bare-metal binary for the host, failing due to host-excluded
hardware dependencies. That attempt is retained in
[the initial failure log](logs/device-example-tests-initial-failure.log).
The tests now run as library unit tests importing the same example helper source;
no firmware dependency was enabled on the host to hide that mismatch.

## Limits and implementation decisions

- These are **lab-only, unencrypted virtual-control** examples. Peer MAC checks
  filter unrelated senders but cannot authenticate them.
- Switch packets use v1 CONTROL/ACK. RC uses the v1 envelope with a lab-specific
  `LAB` + action byte payload; it is not the final production RC wire schema.
- Switch logic uses the existing Pending/VirtualNode; RC uses RcState. Timer
  progress is checked independently of receiving a packet, with a 10ms receive
  timeout. RC does not retry stale updates or depend on per-frame ACK.
- The pinned esp-radio send future must not be canceled and then followed by
  another send on the same instance: a delayed callback can be misattributed.
  The shared helper halts on a 50ms TX-callback timeout rather than reusing the
  canceled sender. That fail-stop is suitable only for these virtual-output
  examples; physical pulse/output safety needs an independent output/watchdog
  design before adapting them to real loads.
- Counter restart/session recovery, authenticated timestamp freshness, pairing,
  encryption and physical pin calibration are not supplied. In a lab session
  reset the pair together; production must establish a new authenticated session.
- No ESP32 hardware was flashed, no radio exchange or application boot was
  verified, and the existing ESP-EMU RTOS blocker was not bypassed or retested.

Customization and future flash instructions are in [examples/README.md](../../examples/README.md).

## Directory organization follow-up

The source examples are grouped into `examples/switch/{client,server}.rs`,
`examples/rc/{client,server}.rs` and `examples/host/loopback.rs`, with shared
transport/codec code in `examples/common/`. Cargo explicitly maps each target
name to its source path, preserving all existing build/run commands and output
filenames. After moving the files, 28 native tests, the host loopback, formatting
and all four firmware example release builds passed again. No hardware checks
were performed for this directory-only change.
