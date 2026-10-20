//! Shared ESP32-C3 transport setup; application logic stays in each example.
#![allow(dead_code)] // Each standalone binary uses one subset of the shared helpers.
pub mod wire;

use embassy_time::{Duration, Instant, with_timeout};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, ram, timer::timg::TimerGroup};
use esp_println::println;
use esp_radio::esp_now::{
    EspNowManager, EspNowReceiver, EspNowSender, EspNowWifiInterface, PeerInfo,
};

// Copy the station MAC printed by each physical board; zero values intentionally refuse startup.
pub const CLIENT_MAC: [u8; 6] = [0; 6];
pub const SWITCH_SERVER_MAC: [u8; 6] = [0; 6];
pub const RC_SERVER_MAC: [u8; 6] = [0; 6];
pub const RF_CHANNEL: u8 = 6;

pub fn now_ms() -> u64 {
    Instant::now().as_millis()
}

pub async fn radio(peer: [u8; 6], label: &str) -> (EspNowManager, EspNowSender, EspNowReceiver) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    println!("NOWCTRL LAB {label}: virtual outputs only, unencrypted lab transport");
    let local = esp_hal::efuse::interface_mac_address(esp_hal::efuse::InterfaceMacAddress::Station);
    println!("Station MAC: {local:?}; peer: {peer:?}; RF channel: {RF_CHANNEL}");
    assert!(
        peer != [0; 6] && peer[0] & 1 == 0 && peer.as_slice() != local.as_bytes(),
        "Configure a nonzero unicast peer MAC in examples/common/mod.rs; it must differ from this board"
    );
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size:64*1024);
    esp_alloc::heap_allocator!(size:36*1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    let wifi = esp_radio::wifi::WifiController::new(peripherals.WIFI, Default::default())
        .expect("Wi-Fi init");
    let radio = wifi.esp_now();
    drop(wifi);
    radio.set_channel(RF_CHANNEL).expect("RF channel");
    let (manager, sender, receiver) = radio.split();
    manager
        .add_peer(PeerInfo {
            interface: EspNowWifiInterface::Station,
            peer_address: peer,
            lmk: None,
            channel: Some(RF_CHANNEL),
            encrypt: false,
        })
        .expect("Add lab peer");
    (manager, sender, receiver)
}

/// Link TX completion is logged separately; only a validated switch ACK completes a request.
pub async fn send(sender: &mut EspNowSender, peer: &[u8; 6], packet: &[u8]) {
    let result = with_timeout(Duration::from_millis(50), sender.send_async(peer, packet)).await;
    match result {
        Ok(status) => println!("LAB link TX {status:?}"),
        // esp-radio cannot safely reuse a sender after canceling its pending send future.
        Err(_) => panic!("Lab TX callback timeout: halt rather than reuse a canceled sender"),
    }
}
