# Software continuation — 2026-10-20 (Asia/Shanghai)

Hardware validation is deferred at the user's request. **P0, P1 and P3 hardware
acceptance gates remain open.** This delivery adds native control/input logic
and an interactive local management preview; it does not finish firmware radio,
pairing, flash storage or embedded HTTP integration.

## Native firmware-independent behavior

- `control`: virtual output defaults OFF, ON/OFF/TOGGLE/PULSE transitions, pulse
  deadlines, target/request checks, one cached result and monotonic request-ID
  rejection. Duplicate PULSE never extends its original deadline; duplicate
  TOGGLE never executes again. Only the newest request's result is cached;
  older requests are rejected as stale, not reexecuted.
- Delivery uses stop-and-wait: a single pending request, three total sends with
  200ms/400ms retry spacing and an 800ms final ACK window. ACKs must match node
  and request, arrive after a send and before final expiry, and become terminal.
  A new session must be authenticated externally; recreate the Node state and
  request counter only as part of that secure transition. Never wrap request IDs
  in the same session. Transport identity/session authentication is a caller duty.
- `input`: normalized button state, configurable 30ms debounce/300ms double-click
  window, and 3s/8s/15s holds. Holds resolve on release, with the longest gesture
  winning. ResetRequested is a request for a guarded confirmation flow, not an
  implemented reset. BOOT GPIO and polarity are not assumed.
- LED patterns compute logical level from elapsed time, including two channel
  pulse groups and finite success/failure/offline/pairing/Web/reset indications.
  A driver must schedule/repeat state indications and map polarity/LED technology;
  no physical GPIO or WS2812 driver is supplied here.

[22 Rust tests](logs/software-host-tests.log) passed, including 100 sequential
TOGGLE requests with each ACK lost until the final attempt, duplicate packet
injection and wrong-node delivery. This is a deterministic **host** test, not
the 100-command two-board acceptance test. [Clippy](logs/software-host-clippy.log)
passed with warnings denied. Both roles still link:
[Controller](logs/software-controller-build.log), [Node](logs/software-node-build.log).

## Management page and local API

Run `python3 tools/dev_server.py --port 8080`, then open `http://127.0.0.1:8080`.
The page follows root DESIGN.md; all assets are local. Software simulation is
prominently labeled, including online and ACK data. The preview binds loopback
only, validates mutation token/Host/Origin/body size, bounds the directory to
eight nodes, applies atomic reorder by stable IDs and guards removal by ID.
Name rendering uses textContent, without HTML interpolation. Unsaved settings
are retained during polling, and unchanged directory rows are not rebuilt.

[9 Python tests](logs/software-api-tests.log) cover reorder rejection/identity,
pairing window/capacity, persistence and safe restart, corrupted storage,
disable/control rejection, pulse expiry/bounds, disk-failure rollback, payload
validation, guarded delete, HTTP assets/envelopes and authentication/origin/Host
rejection. The existing two ESP-EMU checker tests also pass. JavaScript syntax
and Rust formatting checks pass. GitHub Actions includes the new Python checks;
remote CI has not run.

Browser checks on the desktop preview confirmed:

1. Page load and explicit simulation status; three initial example nodes.
2. TOGGLE changes the selected virtual output to ON and records a simulated ACK.
3. A simulated pairing window opens; adding a test node increases the count.
4. Rename is shown in both directory and target, and survives server restart.
5. Reorder moves that same node from CH4 to CH3 without changing its ID.
6. The preview reloads after server restart with outputs OFF, retaining names/order.

[Desktop screenshot](screenshots/management-desktop.png). Responsive CSS includes
640px/1100px breakpoints, but an actual mobile viewport/device was **not** tested.
The delete-dialog return-value issue found in review and the polling row-rebuild
issue found during browser checks were corrected before final verification. API remove behavior is tested;
removal confirmation was not separately exercised in the browser.

## Remaining integration and validation

P0 firmware remains the discovery-only broadcaster; it does not call the new
control/input modules or start an HTTP stack. The management service is a Python
preview adapter, not the Rust core running behind HTTP. Rust assets are available
via `include_str!` for later firmware HTTP embedding. No GUI result here proves
ESP-NOW send/receive, secure enrollment, Flash recovery, SoftAP/HTTP coexistence,
GPIO behavior, power figures or RC failsafe.

Next implementation work: controller/node task integration with authenticated
peer context, actual paired-directory storage adapters and flash recovery,
protected embedded SoftAP/HTTP, then hardware validation when boards are available.
The existing ESP-EMU `esp_rtos::start` blocker is unchanged; no bypass was added.
