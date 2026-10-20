//! P0 discovery experiment based on esp-rs embassy_esp_now_duplex.
//! NO physical outputs, pairing, encryption, or Web configuration yet.
#![no_std]
#![no_main]

#[cfg(all(feature = "controller", feature = "node"))]
compile_error!("Select exactly one of 'controller' and 'node'");
#[cfg(not(any(feature = "controller", feature = "node")))]
compile_error!("Select one of 'controller' or 'node'");

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Ticker};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, ram, timer::timg::TimerGroup};
use esp_println::println;
use esp_radio::esp_now::{BROADCAST_ADDRESS, EspNowReceiver, EspNowSender};

esp_bootloader_esp_idf::esp_app_desc!();

const CHANNEL: u8 = 6;
#[cfg(feature = "controller")]
const ROLE: &str = "controller";
#[cfg(not(feature = "controller"))]
const ROLE: &str = "node";
#[cfg(feature = "controller")]
const HELLO: &[u8] = b"NOWCTRL:DISCOVER";
#[cfg(not(feature = "controller"))]
const HELLO: &[u8] = b"NOWCTRL:NODE";

#[esp_hal::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    println!("NOWCTRL BOOT role={ROLE} rf_channel={CHANNEL}");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    println!("NOWCTRL HAL ready");
    println!(
        "NOWCTRL MAC {:?}",
        esp_hal::efuse::interface_mac_address(esp_hal::efuse::InterfaceMacAddress::Station)
    );
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    println!("NOWCTRL RTOS starting");
    esp_rtos::start(timg0.timer0);

    println!("NOWCTRL RTOS ready");
    spawner.spawn(tick_task().unwrap());
    // Yield before synchronous radio initialization so timer startup is observable.
    embassy_time::Timer::after_secs(3).await;
    println!("NOWCTRL RADIO initializing");
    let controller = esp_radio::wifi::WifiController::new(peripherals.WIFI, Default::default())
        .expect("radio initialization failed");
    let radio = controller.esp_now();
    drop(controller);
    radio
        .set_channel(CHANNEL)
        .expect("channel selection failed");
    println!(
        "NOWCTRL RADIO ready rf_channel={CHANNEL} espnow={:?}",
        radio.version()
    );

    let (_manager, sender, receiver) = radio.split();
    static TX: static_cell::StaticCell<Mutex<NoopRawMutex, EspNowSender>> =
        static_cell::StaticCell::new();
    let sender = TX.init(Mutex::new(sender));
    spawner.spawn(rx_task(receiver).unwrap());
    spawner.spawn(tx_task(sender).unwrap());

    loop {
        embassy_time::Timer::after_secs(60).await;
    }
}

#[embassy_executor::task]
async fn tx_task(sender: &'static Mutex<NoopRawMutex, EspNowSender>) {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut seq = 0u32;
    loop {
        ticker.next().await;
        let result = sender
            .lock()
            .await
            .send_async(&BROADCAST_ADDRESS, HELLO)
            .await;
        println!("NOWCTRL TX role={ROLE} seq={seq} result={result:?}");
        seq = seq.wrapping_add(1);
    }
}

#[embassy_executor::task]
async fn rx_task(mut receiver: EspNowReceiver) {
    loop {
        let packet = receiver.receive_async().await;
        println!("RX from {:?}: {:?}", packet.info.src_address, packet.data());
    }
}

#[embassy_executor::task]
async fn tick_task() {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut tick = 0u32;
    loop {
        ticker.next().await;
        tick = tick.wrapping_add(1);
        println!("NOWCTRL TICK role={ROLE} count={tick}");
    }
}
