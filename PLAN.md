# NowCtrl Development Plan

- [x] Create public-repository project structure and documentation.
- [x] Draft initial ESP32-C3 Rust/Embassy ESP-NOW discovery firmware.
- [ ] P0: Set up Rust riscv32imc-unknown-none-elf + espflash; compile both role variants.
- [ ] P0: Flash two ESP32-C3 boards; confirm bidirectional discovery/channel 6 and restart recovery.
- [ ] P0: Add SoftAP + HTTP proof and verify concurrent ESP-NOW traffic. If incompatible, switch modes.
- [ ] P1: BOOT button handling, LED blink UI, binary control protocol and application ACK with dedupe.
- [ ] P2: Multi-node discovery, physically approved pairing, stable IDs, order and persistent NVS.
- [ ] P3: Authenticated Web UI and mobile configuration through temporary SoftAP.
- [ ] P4: Encrypted unicast, anti-replay, disconnection detection, repeated control testing.
- [ ] P5: Controller/Node power strategies, physical current and latency measurements.
- [ ] P6: GPIO ON/OFF/TOGGLE/PULSE, RC continuous controls and mandatory failsafe.

## Definition of P0 done
Real two-board bidirectional communication and restart recovery, separately verified AP/HTTP, and a documented outcome on ESP-NOW/AP coexistence. Current generated code has not been compiled or flashed.
