//! Virtual RC server. Put application-specific axis handling in on_axes_changed.
#![no_std]
#![no_main]
#[path = "../common/mod.rs"]
mod common;
use common::{CLIENT_MAC, now_ms, wire};
use embassy_executor::Spawner;
use embassy_time::{Duration, with_timeout};
use esp_println::println;
use nowctrl::preset::{Axes, RcError, RcState};
esp_bootloader_esp_idf::esp_app_desc!();

fn on_axes_changed(axes: Axes) {
    println!(
        "APPLICATION virtual throttle={} steering={}",
        axes.throttle, axes.steering
    );
    // No motor/servo GPIO here. Physical calibration, neutral state and secure sessions are not implemented.
}

#[esp_hal::main]
async fn main(_spawner: Spawner) -> ! {
    let (_manager, _sender, mut receiver) = common::radio(CLIENT_MAC, "rc-server").await;
    let mut state = RcState::default();
    let mut last = Axes::default();
    on_axes_changed(last);
    loop {
        if let Ok(packet) = with_timeout(Duration::from_millis(10), receiver.receive_async()).await
            && packet.info.src_address == CLIENT_MAC
        {
            match wire::rc_update(packet.data()) {
                Ok((seq, action)) => match state.update(seq, action, now_ms()) {
                    Ok(()) | Err(RcError::StaleSequence) => {}
                    Err(_) => state.stop(),
                },
                Err(_) => state.stop(),
            }
        }
        // Timer progresses even when no packet arrives; repeats cannot keep a command alive.
        let axes = state.advance(now_ms()).unwrap_or_else(|_| {
            state.stop();
            Axes::default()
        });
        if axes != last {
            last = axes;
            on_axes_changed(axes);
        }
    }
}
