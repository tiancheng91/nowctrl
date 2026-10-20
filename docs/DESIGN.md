# NowCtrl Technical Design

## Architecture
Single Rust no_std/Embassy codebase, two roles compiled separately (Controller/Node), ESP32-C3 Mini boards, fixed ESP-NOW RF channel 6. Radio: esp-radio; MCU: esp-hal; async: esp-rtos and Embassy. On-demand Wi-Fi SoftAP/HTTP with embassy-net and a lightweight HTTP server planned; test radio coexistence first.

## Data model
Stable node ID + MAC + paired credentials; mutable display order (CH1, CH2, ...) and name, state, action policy. Ordering never changes identity. Config stored in versioned Flash with recovery plan.

## Protocol
Wire header: version u8, type u8, flags u16, node_id u32, sequence u32, request_id u32, payload length u16 (little-endian). Keep initial frames under 200 bytes. DISCOVER, PAIR, HELLO, CONTROL, CONTROL_ACK, CONFIG, STATUS. Distinguish radio delivery from business execution; deduplicate control requests. Encryption + anti-replay for production unicast; broadcasts only for discovery.

## Modes
Switch: retry + ACK + deduplication. RC: continuous latest-frame-wins updates and mandatory link-loss neutral state; do not retry stale positional commands. Receiver radio must be awake when responsive control is needed. Deep sleep cannot receive ESP-NOW; battery duty-cycling trades latency for energy.

## Web
Short-lived protected SoftAP on Controller, phone connects to 192.168.4.1, API for nodes, order, names and settings. ESP-NOW/SoftAP concurrency under esp-radio must be verified on real ESP32-C3. Fallback: setup mode suspends remote control.

## P0 implementation
Initial firmware broadcasts role-specific discovery strings each second and logs received frames. This does not pair, authenticate, control GPIO or launch Web AP. Hardware BOOT/LED pinouts vary by board and must be verified.
