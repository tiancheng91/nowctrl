//! Platform-independent protocol code, shared by firmware and native tests.
#![no_std]

pub mod control;
pub mod input;
pub mod preset;
pub mod protocol;

/// Self-contained assets for a future firmware HTTP adapter.
pub mod web_assets {
    pub const INDEX_HTML: &str = include_str!("../web/index.html");
    pub const STYLE_CSS: &str = include_str!("../web/style.css");
    pub const APP_JS: &str = include_str!("../web/app.js");
}

#[cfg(test)]
extern crate self as nowctrl;

#[cfg(test)]
mod example_tests;
