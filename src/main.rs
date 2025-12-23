#![no_std]
#![no_main]

use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io_async::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

// Program metadata
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"LT7689 SD Browser"),
    embassy_rp::binary_info::rp_program_description!(
        c"WiFi-enabled SD card file browser for Raspberry Pi Pico 2W"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

const WIFI_SSID: &str = "PicoW_SD_Browser";
const WIFI_PASSWORD: &str = "12345678";

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn http_server_task(stack: &'static Stack<'static>) {
    info!("HTTP server task started");
    Timer::after(Duration::from_millis(500)).await;
    info!("Starting HTTP server on 192.168.4.1:80");

    let mut rx_buffer = [0; 8192];
    let mut tx_buffer = [0; 8192];
    let mut request_count = 0u32;

    loop {
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(30)));

        info!(
            "Listening on TCP:80... (requests served: {})",
            request_count
        );
        if let Err(e) = socket.accept(80).await {
            warn!("Accept error: {:?}", e);
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        request_count += 1;

        match handle_client(&mut socket).await {
            Ok(_) => info!("Request #{} completed successfully", request_count),
            Err(e) => warn!("Request #{} failed: {:?}", request_count, e),
        }

        socket.abort();
        Timer::after(Duration::from_millis(50)).await;
    }
}

async fn handle_client(socket: &mut TcpSocket<'_>) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = [0; 2048];

    // Read request with timeout
    let n = match embassy_time::with_timeout(Duration::from_secs(5), socket.read(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => {
            warn!("Read error: {:?}", e);
            return Err(e);
        }
        Err(_) => {
            warn!("Read timeout");
            return Ok(());
        }
    };

    if n == 0 {
        info!("Empty request, closing");
        return Ok(());
    }

    let request = core::str::from_utf8(&buf[..n]).unwrap_or("");
    info!("HTTP Request ({} bytes)", n);

    // Parse HTTP request
    if let Some(first_line) = request.lines().next() {
        let parts: heapless::Vec<&str, 3> = first_line.split_whitespace().collect();
        if parts.len() >= 2 {
            let method = parts[0];
            let path = parts[1];
            info!("Method: {}, Path: {}", method, path);

            // Send HTTP response
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\n").await;
            let _ = socket.write_all(b"Content-Type: text/html; charset=utf-8\r\n").await;
            let _ = socket.write_all(b"Connection: close\r\n").await;
            let _ = socket.write_all(b"\r\n").await;

            // HTML content
            let _ = socket.write_all(b"<!DOCTYPE html>\n").await;
            let _ = socket.write_all(b"<html>\n<head>\n").await;
            let _ = socket.write_all(b"<title>Pico 2W SD Card Browser</title>\n").await;
            let _ = socket.write_all(b"<meta name='viewport' content='width=device-width, initial-scale=1'>\n").await;
            let _ = socket.write_all(b"<meta http-equiv='refresh' content='5'>\n").await;
            let _ = socket.write_all(b"<style>\n").await;
            let _ = socket.write_all(b"body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }\n").await;
            let _ = socket.write_all(b"h1 { color: #333; }\n").await;
            let _ = socket.write_all(b".container { max-width: 900px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }\n").await;
            let _ = socket.write_all(b".status { background: #e8f5e9; padding: 15px; border-radius: 5px; margin: 20px 0; border-left: 4px solid #4caf50; }\n").await;
            let _ = socket.write_all(b"ul { list-style: none; padding: 0; }\n").await;
            let _ = socket.write_all(b"li { padding: 12px; margin: 8px 0; background: #fafafa; border-radius: 5px; border-left: 3px solid #2196f3; }\n").await;
            let _ = socket.write_all(b".info { color: #666; font-size: 0.9em; margin-top: 30px; padding-top: 20px; border-top: 2px solid #eee; }\n").await;
            let _ = socket.write_all(b".hw-info { background: #fff3cd; padding: 10px; border-radius: 5px; margin: 10px 0; }\n").await;
            let _ = socket.write_all(b"</style>\n</head>\n<body>\n").await;
            let _ = socket.write_all(b"<div class='container'>\n").await;
            let _ = socket.write_all(b"<h1>\xF0\x9F\x97\x82\xEF\xB8\x8F SD Card File Browser</h1>\n").await;
            let _ = socket.write_all(b"<p>Running on <strong>Raspberry Pi Pico 2W</strong> (RP2350)</p>\n").await;
            let _ = socket.write_all(b"<div class='status'>\n").await;
            let _ = socket.write_all(b"<strong>\xE2\x9C\x85 WiFi AP Active:</strong> ").await;
            let _ = socket.write_all(WIFI_SSID.as_bytes()).await;
            let _ = socket.write_all(b"<br><strong>\xE2\x9C\x85 IP Address:</strong> 192.168.4.1\n").await;
            let _ = socket.write_all(b"<br><strong>\xE2\x9C\x85 Web Server:</strong> Running on port 80\n").await;
            let _ = socket.write_all(b"</div>\n").await;

            let _ = socket.write_all(b"<h2>Files on SD Card:</h2>\n").await;
            let _ = socket.write_all(b"<ul>\n").await;
            let _ = socket.write_all(b"<li>\xF0\x9F\x93\x84 README.txt <span style='color:#999'>(1.2 KB)</span></li>\n").await;
            let _ = socket.write_all(b"<li>\xF0\x9F\x93\x84 config.json <span style='color:#999'>(456 bytes)</span></li>\n").await;
            let _ = socket.write_all(b"<li>\xF0\x9F\x93\x81 data/ <span style='color:#999'>(directory)</span></li>\n").await;
            let _ = socket.write_all(b"<li>\xF0\x9F\x93\x84 log.txt <span style='color:#999'>(3.4 KB)</span></li>\n").await;
            let _ = socket.write_all(b"<li>\xF0\x9F\x93\x84 sensor_data.csv <span style='color:#999'>(8.9 KB)</span></li>\n").await;
            let _ = socket.write_all(b"<li>\xF0\x9F\x93\x84 photos.zip <span style='color:#999'>(245 KB)</span></li>\n").await;
            let _ = socket.write_all(b"</ul>\n").await;

            let _ = socket.write_all(b"<div class='hw-info'>\n").await;
            let _ = socket.write_all(b"<strong>\xE2\x9A\xA0\xEF\xB8\x8F Note:</strong> SD card reading implementation in progress.\n").await;
            let _ = socket.write_all(b"The files shown above are sample data.\n").await;
            let _ = socket.write_all(b"</div>\n").await;

            let _ = socket.write_all(b"<div class='info'>\n").await;
            let _ = socket.write_all(b"<p><strong>Current Status:</strong></p>\n").await;
            let _ = socket.write_all(b"<ul>\n").await;
            let _ = socket.write_all(b"<li>\xE2\x9C\x85 WiFi Access Point: Active</li>\n").await;
            let _ = socket.write_all(b"<li>\xE2\x9C\x85 HTTP Server: Running</li>\n").await;
            let _ = socket.write_all(b"<li>\xE2\x9C\x85 SPI Interface: Initialized</li>\n").await;
            let _ = socket.write_all(b"<li>\xE2\x8F\xB3 SD Card Reader: Coming soon</li>\n").await;
            let _ = socket.write_all(b"</ul>\n").await;

            let _ = socket.write_all(b"<p><strong>Hardware Configuration:</strong></p>\n").await;
            let _ = socket.write_all(b"<ul>\n").await;
            let _ = socket.write_all(b"<li><strong>MCU:</strong> RP2350A (Dual Cortex-M33 @ 150MHz)</li>\n").await;
            let _ = socket.write_all(b"<li><strong>WiFi:</strong> CYW43439 (2.4GHz 802.11n)</li>\n").await;
            let _ = socket.write_all(b"<li><strong>SD Card SPI:</strong> CLK=GP2, MOSI=GP3, MISO=GP0, CS=GP5</li>\n").await;
            let _ = socket.write_all(b"</ul>\n").await;

            let _ = socket.write_all(b"<p style='color:#666;font-size:0.85em;margin-top:20px'>\n").await;
            let _ = socket.write_all(b"<strong>Instructions:</strong><br>\n").await;
            let _ = socket.write_all(b"1. Connect SD card module to Pico 2W using pins listed above<br>\n").await;
            let _ = socket.write_all(b"2. Format SD card as FAT32<br>\n").await;
            let _ = socket.write_all(b"3. Add files to SD card<br>\n").await;
            let _ = socket.write_all(b"4. Files will be listed here when SD reading is implemented<br>\n").await;
            let _ = socket.write_all(b"</p>\n").await;
            let _ = socket.write_all(b"</div>\n").await;

            let _ = socket.write_all(b"<p style='text-align:center;color:#999;font-size:0.8em;margin-top:30px'>\n").await;
            let _ = socket.write_all(b"LT7689 - Page auto-refreshes every 5 seconds\n").await;
            let _ = socket.write_all(b"</p>\n").await;
            let _ = socket.write_all(b"</div>\n</body>\n</html>\r\n").await;

            info!("Response sent successfully");
        }
    }

    Timer::after(Duration::from_millis(100)).await;
    Ok(())
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting LT7689 - Pico 2W SD Card Browser");
    let p = embassy_rp::init(Default::default());

    // Initialize WiFi firmware blobs
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // Initialize CYW43 WiFi chip
    info!("Initializing CYW43 WiFi chip...");
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(cyw43_task(runner).unwrap());

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::Performance)
        .await;

    info!("CYW43 initialized successfully");

    // Initialize SD Card SPI (for future implementation)
    info!("Initializing SD card SPI interface...");
    let mut sd_config = SpiConfig::default();
    sd_config.frequency = 400_000; // Start slow for SD card init

    let spi_bus = Spi::new(
        p.SPI0,
        p.PIN_2,  // CLK
        p.PIN_3,  // MOSI
        p.PIN_0,  // MISO
        p.DMA_CH1,
        p.DMA_CH2,
        sd_config,
    );

    let spi_cs = Output::new(p.PIN_5, Level::High);
    let _spi_device = ExclusiveDevice::new(spi_bus, spi_cs, embassy_time::Delay);
    info!("SD card SPI interface initialized (reading implementation pending)");

    // Configure network stack for AP mode with static IP
    info!("Configuring network stack...");
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 4, 1), 24),
        gateway: Some(embassy_net::Ipv4Address::new(192, 168, 4, 1)),
        dns_servers: heapless::Vec::new(),
    });

    let seed = 0x0123_4567_89ab_cdef;

    static STACK: StaticCell<Stack<'static>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<16>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<16>::new()),
        seed,
    );
    let stack = STACK.init(stack);

    spawner.spawn(net_task(runner).unwrap());

    // Start WiFi AP
    info!("Starting WiFi Access Point...");
    info!("SSID: {}, Password: {}", WIFI_SSID, WIFI_PASSWORD);

    control.start_ap_wpa2(WIFI_SSID, WIFI_PASSWORD, 5).await;
    info!("WiFi AP started successfully!");
    info!("Connect to WiFi: {}", WIFI_SSID);
    info!("Then browse to: http://192.168.4.1");

    // Wait for network stack to be ready
    Timer::after(Duration::from_secs(2)).await;
    info!("Network stack ready");

    // Spawn HTTP server
    info!("Starting HTTP server task...");
    spawner.spawn(http_server_task(stack).unwrap());
    info!("HTTP server task spawned successfully");

    // Blink LED to indicate system is running
    info!("System ready! LED blinking to indicate AP is active.");
    loop {
        control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(100)).await;
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(900)).await;
    }
}
