# NowCtrl Product Requirements

## Vision
An open-source local wireless control system for switches and RC models, based on ESP32-C3, ESP-NOW, Rust, and Embassy; no router or cloud needed.

## Roles
Controller: manage nodes/channels; BOOT button single-click next channel, double-click send, long press pairing / Web setup; LED blinks channel number. Node: listen for authorized commands, acknowledge execution, apply output configuration (future).

## Modes
- Switch: ON/OFF/TOGGLE/PULSE, ACK, retry and request deduplication.
- RC: continuous latest-value controls, low-latency, failsafe on radio loss, no replay of stale actions. Never use deep-sleep listening for active RC mode.
- Setup: short-lived WPA2 SoftAP with HTTP UI for pairing, names, channel ordering and configuration.

## MVP
P0 dual-board ESP-NOW proof; P1 reliable controls and buttons/LED; P2 multi-node and persistent pairing; P3 HTTP setup; P4 security; P5 power; P6 outputs/RC.

## Security & Safety
Never actuate unpaired nodes; MAC alone is not authentication. Pairing requires physical authorization and encrypted unicast. All physical outputs must fail safe on reboot or loss of control. P0 is discovery-only and is not ready for real actuators.
