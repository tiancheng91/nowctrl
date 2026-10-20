# Device-preset software validation — 2026-10-20

The user confirmed that only undefined-device GPIO output must be read-only;
defined switch and model/toy RC presets remain operable from the management page.

## Implemented

| Profile | Page behavior | API boundary |
| --- | --- | --- |
| switch | ON/OFF/TOGGLE/PULSE | Switch control only |
| rc_basic | Held forward/boost/reverse/left/right, STOP and logical axes | Increasing RC sequences; no switch operations |
| unknown / missing | GPIO HIGH/LOW/unknown, no output buttons | Neither switch nor RC writes; no generic GPIO-write route |

Name/order/enabled metadata remains editable. Type is immutable after creation;
only the mock creation form declares a simulated type. Old host v1 records load
as unknown/read-only without changing IDs or overwriting the original file on
read. Successful later saves write v2. RC and GPIO runtime state are not persisted.

RC preview values: forward +35%, boost +65%, reverse -30% throttle; left/right
are steering only (-100%/+100%, throttle zero). Stop neutralizes both axes. All
values are logical software presets, not PWM calibration or motor mixing.
The UI refreshes held frames at ~100ms, requests STOP on release/cancel/blur/
visibility/page-exit/node/config changes and does not retry failed RC frames.
The model's background loop checks its 300ms watchdog every 50ms; the no_std core
exposes the same timeout for a future periodic firmware task to drive. Sequence
replays cannot extend the watchdog. Transport authentication, sender freshness
and real output neutral/failsafe remain unimplemented physical-integration gates.

## Evidence

- [25 native Rust tests](logs/preset-host-tests.log) pass, including bounded preset
  mappings, unknown permissions, stale/duplicate sequences, watchdog expiry,
  explicit stop and backward-clock rejection.
- [12 API tests](logs/preset-api-tests.log) pass, including unknown write rejection,
  cross-profile action rejection, immutable types, legacy migration, v2 restoration,
  valid/invalid RC actions, monotonic sequence checks, timeout and disable-to-stop.
- [Clippy](logs/preset-clippy.log), JS syntax, formatting and both firmware builds
  pass: [Controller](logs/preset-controller-build.log), [Node](logs/preset-node-build.log).
- Browser checks confirmed a legacy node displays read-only HIGH/LOW/unknown with
  no output controls; creating a simulated RC node displays all six actions.
  A 700ms forward hold produced increasing sequences and a STOP on release;
  boost hold/release was also exercised. Final reported throttle and steering
  are both zero. This was observed in the narrow responsive browser panel,
  not on a physical phone or controller.

Screenshots: [GPIO read-only](screenshots/gpio-readonly.png),
[RC preset](screenshots/rc-preset.png). Local preview remains at
`http://127.0.0.1:8080` (`python3 tools/dev_server.py --port 8080`).

Current firmware still broadcasts P0 discovery strings. No RC_UPDATE payload
schema, real radio update path, paired session reset, calibrated GPIO/PWM or
actual model operation is claimed. The demo Python adapter and Rust core are
independently tested implementations. No commit/push or physical testing occurred.
