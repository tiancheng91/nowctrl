//! Automatic virtual RC client. Replace this sequence with your own input source.
#![no_std]
#![no_main]
#[path = "../common/mod.rs"]
mod common;
use common::{RC_SERVER_MAC, now_ms, wire};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use nowctrl::{preset::RcAction, protocol::MAX_FRAME_LEN};
esp_bootloader_esp_idf::esp_app_desc!();

const ACTIONS: [(RcAction, u64); 10] = [
    (RcAction::Forward, 600),
    (RcAction::Stop, 400),
    (RcAction::BoostForward, 600),
    (RcAction::Stop, 400),
    (RcAction::Reverse, 600),
    (RcAction::Stop, 400),
    (RcAction::Left, 600),
    (RcAction::Stop, 400),
    (RcAction::Right, 600),
    (RcAction::Stop, 800),
];

#[esp_hal::main]
async fn main(_spawner: Spawner) -> ! {
    let (_manager, mut sender, _receiver) = common::radio(RC_SERVER_MAC, "rc-client").await;
    let mut seq = 0u32;
    loop {
        for (action, hold_ms) in ACTIONS {
            let until = now_ms() + hold_ms;
            while now_ms() < until {
                seq = seq
                    .checked_add(1)
                    .expect("Start a new session before sequence exhaustion");
                let mut packet = [0; MAX_FRAME_LEN];
                let len = wire::rc_packet(action, seq, &mut packet).unwrap();
                common::send(&mut sender, &RC_SERVER_MAC, &packet[..len]).await;
                // Always send a fresh frame; no retry/ACK dependency for an old RC update.
                Timer::after(Duration::from_millis(100)).await;
            }
        }
    }
}
