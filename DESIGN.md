---
name: ESP-EMU DevStudio
description: Dense, terminal-adjacent dark system designed for in-browser hardware emulators, embedded registers, and low-level firmware telemetry.
colors:
  primary: "#ff8c00"
  on-primary: "#000000"
  primary-container: "#3d2200"
  secondary: "#00e5ff"
  on-secondary: "#000000"
  neutral: "#0d0e11"
  surface: "#16181d"
  surface-variant: "#1f232b"
  outline: "#2c313a"
  outline-variant: "#3b424e"
  text-primary: "#e6edf3"
  text-secondary: "#8b949e"
  text-muted: "#545d68"
  status-success: "#2ea043"
  status-error: "#f85149"
  status-warning: "#d29922"
typography:
  fontFamily:
    sans: "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
    mono: "'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace"
  heading-sm:
    fontFamily: "{typography.fontFamily.sans}"
    fontSize: "0.75rem"
    fontWeight: "700"
    letterSpacing: "0.08em"
    textTransform: "uppercase"
  heading-md:
    fontFamily: "{typography.fontFamily.sans}"
    fontSize: "0.875rem"
    fontWeight: "600"
    lineHeight: "1.25"
  body-sm:
    fontFamily: "{typography.fontFamily.sans}"
    fontSize: "0.75rem"
    lineHeight: "1.3"
  body-md:
    fontFamily: "{typography.fontFamily.sans}"
    fontSize: "0.8125rem"
    lineHeight: "1.4"
  code-xs:
    fontFamily: "{typography.fontFamily.mono}"
    fontSize: "0.6875rem"
    lineHeight: "1.2"
  code-sm:
    fontFamily: "{typography.fontFamily.mono}"
    fontSize: "0.75rem"
    lineHeight: "1.35"
spacing:
  2xs: "2px"
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
rounded:
  none: "0px"
  xs: "2px"
  sm: "4px"
  md: "6px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.heading-sm}"
    rounded: "{rounded.xs}"
    padding: "4px 10px"
  button-primary-hover:
    backgroundColor: "#ffa333"
  button-control:
    backgroundColor: "{colors.surface-variant}"
    textColor: "{colors.text-primary}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.xs}"
    padding: "4px 8px"
  card-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.xs}"
    padding: "10px"
  input-text:
    backgroundColor: "{colors.neutral}"
    textColor: "{colors.text-primary}"
    typography: "{typography.code-sm}"
    rounded: "{rounded.xs}"
    padding: "4px 8px"
  terminal-window:
    backgroundColor: "#08090a"
    textColor: "{colors.text-primary}"
    typography: "{typography.code-sm}"
    rounded: "{rounded.none}"
    padding: "10px"
---

The visual architecture evokes an **industrial hardware workbench** — compact, high-contrast, strictly organized, and optimized for telemetry clarity, register grids, and continuous serial monitoring.

## Colors
The palette is built around deep circuit-grade neutrals with electric phosphor accents to maximize readability under zero-glare, prolonged debugging sessions.

- **Primary (`#ff8c00`):** Core hardware state color (Run, execution active, hardware pin assertion). High energy, instant visual target.
- **Secondary (`#00e5ff`):** Terminal prompt indicators, network handshakes, and instruction pointer updates (`pc 0x--------`).
- **Neutral Canvas (`#0d0e11`):** Deep black chassis substrate.
- **Surfaces (`#16181d`, `#1f232b`):** Modular component container layers providing visual hierarchy without drop shadows.
- **Outlines (`#2c313a`, `#3b424e`):** 1px mechanical panel dividers separating registers, memory addresses, and config widgets.
- **Status Accents:**
  - `status-success` (`#2ea043`): Active emulator loop, live link.
  - `status-error` (`#f85149`): Core panic, broken connection, parse failure.
  - `status-warning` (`#d29922`): Target reset state, baud mismatches.

## Typography
Dense dual-font structure separating micro-control labels from fixed-pitch register columns:

- **Monospace (`JetBrains Mono`, `Fira Code`):** The workhorse font. Used for the entire lower dashboard (Register tables, 32-bit hex values `00000000`, GPIO pin numbers, UART logs, and cycle counters). Enforces tabular alignment across multi-column data views.
- **Sans-Serif (`Inter`):** Reserved for top-level window controls, card section headers, form inputs (SSID, Baud, WebSocket URL), and metadata tooltips.
- **Hierarchy:** Section headers (`heading-sm`) use strict uppercase letter-spaced styling (`0.08em`) to act as functional panel labels rather than prose headings.

## Layout & Spatial System
Information density is calibrated for desktop developer tooling:

- **Grid Strategy:** Fixed multi-column technical layout. Left column handles target configuration and network bridge inputs; center column holds high-velocity UART terminal stream; right column holds live register state tables, CPU cores, performance gauges, and GPIO matrix.
- **Spacing:** Micro-spacers (`2px`, `4px`, `8px`) predominate to fit complete microcontroller register maps (x0 to x31) above the fold.
- **Panel Borders:** Panels do not float with ambient blur; they are strictly framed with 1px border rules (`#2c313a`) with squared or 2px micro-radii.

## Component Patterns

- **Control Buttons:** Small, mechanical action blocks (`padding: 4px 10px`, `rounded-xs`). Primary actions like `Run` use full amber fill; secondary actions (`Step`, `Pause`, `Reset`) use outlined surface styles.
- **Register & Memory Matrix:** Clean 2-column or 4-column key-value tables where keys (`x0`, `x16`) are muted gray and values (`00000000`) are high-contrast white monospaced digits.
- **GPIO Pin Array:** Compact indexed status cells representing individual pin states (High/Low/Input/Output) via fill colors rather than verbose text.
- **UART Terminal:** High-contrast retro terminal canvas with amber/cyan system prefix tags (`[System]`) and full ANSI character support.
