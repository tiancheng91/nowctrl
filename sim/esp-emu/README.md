# ESP32-C3 P0 emulator path

This runs the actual Rust firmware, not a host protocol simulation. No physical
output pin is selected. Both roles broadcast discovery only, without pairing or
actuation. The independent one-second Embassy tick starts before radio setup;
a three-second startup observation interval is used on both emulator and hardware.

## Tools

- Rust: version in `rust-toolchain.toml`, with `riscv32imc-unknown-none-elf`.
- Python 3 (standard library only).
- espflash **4.4.0**: `cargo install espflash --version 4.4.0 --locked`.
- ESP-EMU **0.49.0**, native macOS Apple Silicon build.

Download and inspect the [official installer](https://github.com/espressif/esp-emulator/blob/main/install.sh),
then run it with `--version 0.49.0`. The default installation directory is
`$HOME/.local/bin`. Put that directory on PATH or set `ESP_EMU_BIN` to the
executable. `ESPFLASH_BIN` similarly overrides espflash. No automatic installation
or self-update occurs in the scripts; mismatched versions fail explicitly.

## Build and run

From the repository root:

```sh
python3 sim/esp-emu/p0.py build controller
python3 sim/esp-emu/p0.py build node
python3 sim/esp-emu/p0.py run controller --seconds 20
python3 sim/esp-emu/p0.py run node --seconds 20
```

Each role keeps its own `firmware.elf`, `merged.bin`, and `emulator.log` under
`target/p0/<role>/`. The build uses `--locked --release`, adding
`--no-default-features --features node` for Node. The image tool generates the
bootloader and default partition table, merges the ELF application, and checks
bootloader magic at `0x0`, partition entries at `0x8000`, and application magic
at `0x10000`. The default flash profile is 4 MB; this is an emulator assumption,
not confirmation of the physical board's flash size.

`run` starts a normal ROM/bootloader boot (no skip flags), explicitly selects
`--console usb-jtag` for esp-println auto transport, records stdout/stderr,
and enforces a wall-clock deadline even if the emulator stops responding. It
requires a successful emulator exit, the selected role's startup marker, and
three consecutive ticks, and rejects recognized firmware panic markers.
A failed smoke test returns nonzero and preserves the log. Inspect an existing
log with `python3 sim/esp-emu/p0.py check controller` (or `node`).

A smoke pass verifies startup and async timer progress only. Check
`NOWCTRL RADIO initializing`, `NOWCTRL RADIO ready`, TX status and RX separately.
Neither a ready marker nor a successful send proves peer reception. There is no
HTTP firmware in this iteration, and no assumption of an ESP-NOW bridge between
emulator instances. Physical radio, GPIO/LED and AP coexistence need their own
validation. See [P0 evidence](../../docs/validation/p0.md).

Host checks for image/log validation: `python3 -m unittest discover -s sim/esp-emu -p 'test_*.py'`.
