//! Automatic virtual switch client: edit COMMANDS to customize your application.
#![no_std]
#![no_main]
#[path = "../common/mod.rs"]
mod common;
use common::{SWITCH_SERVER_MAC, now_ms, wire};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, with_timeout};
use esp_println::println;
use nowctrl::{
    control::{Delivery, Pending, Request},
    protocol::{Control, MAX_FRAME_LEN},
};
esp_bootloader_esp_idf::esp_app_desc!();

const COMMANDS: [Control; 4] = [
    Control::On,
    Control::Off,
    Control::Toggle,
    Control::Pulse { duration_ms: 500 },
];

#[esp_hal::main]
async fn main(_spawner: Spawner) -> ! {
    let (_manager, mut sender, mut receiver) =
        common::radio(SWITCH_SERVER_MAC, "switch-client").await;
    let mut request_id = 0u32;
    loop {
        for command in COMMANDS {
            request_id = request_id
                .checked_add(1)
                .expect("Start a new session before request ID exhaustion");
            let request = Request {
                node_id: wire::SWITCH_NODE_ID,
                request_id,
                command,
            };
            let mut pending = Pending::new(request, now_ms()).unwrap();
            loop {
                match pending.poll(now_ms()).unwrap() {
                    Delivery::Send(request) => {
                        let mut packet = [0; MAX_FRAME_LEN];
                        let len =
                            wire::switch_packet(request.command, request.request_id, &mut packet)
                                .unwrap();
                        common::send(&mut sender, &SWITCH_SERVER_MAC, &packet[..len]).await;
                    }
                    Delivery::Complete(result) => {
                        println!("Application ACK id={request_id}: {result:?}");
                        break;
                    }
                    Delivery::TimedOut => {
                        println!("Application ACK timeout id={request_id}");
                        break;
                    }
                    Delivery::Waiting => {}
                }
                if let Ok(packet) =
                    with_timeout(Duration::from_millis(10), receiver.receive_async()).await
                {
                    // A MAC allowlist is only a lab filter, not authentication.
                    if packet.info.src_address != SWITCH_SERVER_MAC {
                        continue;
                    }
                    if let Ok((id, result)) = wire::switch_ack(packet.data()) {
                        pending.acknowledge(wire::SWITCH_NODE_ID, id, result, now_ms());
                    }
                }
            }
            Timer::after_secs(2).await;
        }
    }
}
