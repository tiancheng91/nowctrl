//! Virtual switch server. Put your application reaction in on_output_changed.
#![no_std]
#![no_main]
#[path = "../common/mod.rs"]
mod common;
use common::{CLIENT_MAC, now_ms, wire};
use embassy_executor::Spawner;
use embassy_time::{Duration, with_timeout};
use esp_println::println;
use nowctrl::{
    control::VirtualNode,
    protocol::{ControlAck, MAX_FRAME_LEN},
};
esp_bootloader_esp_idf::esp_app_desc!();

fn on_output_changed(on: bool) {
    println!("APPLICATION virtual switch = {on}");
    // Keep virtual. Add a board-specific output adapter only after secure transport and bench validation.
}

#[esp_hal::main]
async fn main(_spawner: Spawner) -> ! {
    let (_manager, mut sender, mut receiver) = common::radio(CLIENT_MAC, "switch-server").await;
    let mut node = VirtualNode::new(wire::SWITCH_NODE_ID).unwrap();
    let mut last_output = false;
    on_output_changed(false);
    loop {
        if let Ok(packet) = with_timeout(Duration::from_millis(10), receiver.receive_async()).await
            && packet.info.src_address == CLIENT_MAC
            && let Ok((request_id, command)) = wire::switch_command(packet.data())
        {
            let result = node
                .apply(wire::SWITCH_NODE_ID, request_id, command, now_ms())
                .unwrap_or(ControlAck::Rejected);
            if node.output() != last_output {
                last_output = node.output();
                on_output_changed(last_output);
            }
            let mut reply = [0; MAX_FRAME_LEN];
            let len = wire::ack_packet(request_id, result, &mut reply).unwrap();
            common::send(&mut sender, &CLIENT_MAC, &reply[..len]).await;
        }
        node.advance(now_ms()).unwrap();
        if node.output() != last_output {
            last_output = node.output();
            on_output_changed(last_output);
        }
    }
}
