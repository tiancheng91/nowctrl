# NowCtrl

A lightweight ESP-NOW wireless control system for switches, RC models, and other DIY projects, built with Rust and Embassy.

> Status: P0 experimental prototype. Do not connect motors or hazardous loads.

## Features (roadmap)
- ESP32-C3 Controller + multiple Nodes
- ESP-NOW device discovery, pairing, logical channel ordering
- BOOT button switching and LED feedback
- On-demand SoftAP and HTTP management
- Switch actions and RC continuous input with explicit failsafe
- Power management and GPIO extensions

## Development
The P0 firmware is based on the upstream [esp-rs Embassy ESP-NOW duplex example](https://github.com/esp-rs/esp-hal/tree/main/examples/esp-now/embassy_esp_now_duplex). See docs/PRODUCT.md, docs/DESIGN.md and PLAN.md.

The firmware is not yet compiled or tested on hardware. MIT license.
