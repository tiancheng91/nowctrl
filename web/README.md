# NowCtrl management workbench

An offline, dependency-free HTML/CSS/JavaScript management page following the
root `DESIGN.md`: circuit-dark panels, orange actions, cyan communication accents,
compact typography, fine mechanical borders and a three-column desktop layout.
CSS stacks the panels on narrow screens. Fonts use local fallbacks; no CDN,
external assets, build server or Internet connection is required.

## Run the software preview

```sh
python3 tools/dev_server.py --port 8080
```

Open http://127.0.0.1:8080 . The server binds only to loopback. It is a **software
simulator**, not an ESP32 SoftAP or radio bridge. The page always displays that
boundary, and rejects a backend that does not explicitly identify as simulation.
It supports virtual node creation in a 60-second simulated pairing window,
rename/enable, full-list atomic reorder, confirmed removal, switch control,
pulse preview and a bounded event log. Maximum eight virtual nodes.

Configuration persists in `target/web-demo.json`, overridable with `--state`.
Writes use a temporary file, fsync and atomic replace. Only node IDs, names,
enabled flags and order are restored. Outputs restart OFF; online status is
simulated from enabled flags rather than loaded from a saved radio status.
Malformed files cause startup to fail without overwriting the original.
This is host file persistence, not firmware flash persistence or power-loss proof.

Mutations require the per-process random `X-NowCtrl-Token` from status. Host and
Origin checks block cross-origin writes/rebinding. There is no CORS allowance.
This protects the local preview; it is not a production pairing/authentication
implementation. The service has no physical output or radio capabilities.

## API adapter boundary

The preview uses the SPEC `/api/v1` `{ok,data,error}` response envelope and these
planned paths: status, nodes, pairing/start, nodes/order, PATCH/DELETE node, and
node/control. Documented preview request bodies:

- Reorder: `{ "node_ids": [101, 102, 103] }` (all IDs exactly once).
- Update: `{ "name": "工作台灯", "enabled": true }` (either field or both).
- Remove: `{ "confirm_id": 101 }` (must match the URL node).
- Control: `{ "action": "PULSE", "duration_ms": 500 }`; actions OFF/ON/TOGGLE/PULSE.
  The preview policy caps pulse duration at 60,000 ms.

Preview-only extensions: `POST /api/v1/demo/nodes` with `{ "name": "测试节点" }`
and `GET /api/v1/demo/events`. Status adds `mode: simulation`,
`firmware_connected: false` and a mutation token; nodes identify simulated data.
The firmware API contract, flash storage, unique credentials and physical
pairing are still separate implementation work. Do not expose demo pairing
routes on a production controller.

The Rust library exposes the three assets through `nowctrl::web_assets` using
`include_str!` so a future embedded HTTP adapter needs no filesystem. Current
P0 firmware has no HTTP task and does not serve these assets yet. Python's
virtual backend is a UI preview adapter, not execution of the Rust control core.
The Rust retry/dedup core and Python UI flows are tested independently.

## Checks

```sh
python3 -m unittest discover -s tools -p 'test_*.py'
node --check web/app.js
```

See `docs/validation/software.md` for native/HTTP/browser evidence and limits.

## Device presets

A node declares immutable `device_type` as `switch`, `rc_basic`, or `unknown`.
The production source will be the device capability/profile; it is not inferred
from the name. The simulated add-node form can declare a profile at creation;
PATCH cannot change that profile later.

- **Switch**: existing OFF/ON/TOGGLE/PULSE controls.
- **RC basic**: STOP, FORWARD (35%), BOOST_FORWARD (65%), REVERSE (-30%), LEFT and
  RIGHT (steering -100%/+100%, throttle zero). These are logical preview values,
  not calibrated PWM/servo outputs. Left/right do not imply propulsion or a
  differential motor mix. Combined throttle/steering inputs are a later extension.
- **Unknown**: read-only GPIO telemetry. HIGH, LOW and missing/unknown are separate
  states. Neither control endpoint accepts unknown devices, and no GPIO-write
  endpoint exists. Name/order/enabled metadata can still be managed.

RC preview transport is `POST /api/v1/nodes/{id}/rc` with
`{"action":"FORWARD","seq":1}`. It is independent of switch request/ACK/retry
semantics. Hold actions update at about 100ms intervals, never retry stale frames;
release, pointer cancellation, blur, visibility change, page exit, node change
and configuration mutations request STOP. A background 50ms watchdog loop
neutralizes the virtual RC axes after 300ms without a fresh increasing sequence.
The Rust core exposes equivalent logical action mapping and receiver watchdog.
Firmware RC transport/session freshness and physical failsafe are still pending.

Demo storage is now v2 with `device_type`. Existing v1 records lacking a device
type migrate in memory to `unknown` (read-only outputs), preserving IDs, order,
names and enabled flags. The source file is not rewritten on read; the next
successful configuration save writes v2 atomically. Runtime GPIO/RC state is not
restored as online/active state on restart. Example GPIO 0/1/2 values are explicitly
simulated and must never be used as an assumed ESP32-C3 board pinout.
