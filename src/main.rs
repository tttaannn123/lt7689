#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StackResources};
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_rp::peripherals::{SPI0, PIN_23, PIN_24, PIN_25, PIN_29};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeManager};
use {defmt_rtt as _, panic_probe as _};

// Dummy timesource for SD card
struct DummyTime;
impl TimeSource for DummyTime {
    fn get_timestamp(&self) -> Timestamp { Timestamp::from_fat(0, 0) }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // --- SD CARD SETUP ---
    let mut sd_config = SpiConfig::default();
    sd_config.frequency = 4_000_000; // 4MHz
    let spi = Spi::new_blocking(p.SPI0, p.PIN_2, p.PIN_3, p.PIN_0, sd_config);
    let spi_dev = ExclusiveDevice::new(spi, Output::new(p.PIN_5, Level::High), embassy_time::Delay);
    let mut sdcard = SdCard::new(spi_dev, embassy_time::Delay);
    let mut volume_mgr = VolumeManager::new(sdcard, DummyTime);

    // --- WIFI AP SETUP ---
    // (Pico 2W specific: CYW43 firmware and PIO setup omitted for brevity)
    // Assume `stack` is an initialized embassy-net Stack in AP mode
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 4, 1), 24),
        gateway: None,
        dns_servers: Vec::new(),
    });

    // ... Initialize cyw43 and stack here ...

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        if let Err(e) = socket.accept(80).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        // Generate directory listing
        let mut response = heapless::String::<1024>::new();
        response.push_str("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<h1>SD Card Files</h1><ul>").unwrap();

        let _ = volume_mgr.open_root_volume().and_then(|volume| {
            volume_mgr.open_root_dir(&volume).and_then(|root| {
                volume_mgr.iterate_dir(&root, |entry| {
                    let _ = response.push_str("<li>");
                    let _ = response.push_str(entry.name.as_str());
                    let _ = response.push_str("</li>");
                })
            })
        });

        response.push_str("</ul>").unwrap();
        let _ = socket.write_all(response.as_bytes()).await;
        socket.close();
    }
}
