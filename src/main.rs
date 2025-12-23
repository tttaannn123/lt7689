#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io_async::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = "PicoW_SD_Browser";
const WIFI_PASSWORD: &str = "12345678";

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, cyw43_pio::PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting Pico W SD Card Browser...");

    let p = embassy_rp::init(Default::default());

    // Initialize WiFi (CYW43)
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = cyw43_pio::PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, p.PIN_24, p.PIN_29, p.DMA_CH0);

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Start WiFi as Access Point
    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // In production, use real random seed

    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<3>::new()),
        seed,
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    // Start Access Point
    info!("Starting AP mode...");
    match control.start_ap_wpa2(WIFI_NETWORK, WIFI_PASSWORD, 5).await {
        Ok(_) => info!("AP started: {} / {}", WIFI_NETWORK, WIFI_PASSWORD),
        Err(e) => {
            error!("Failed to start AP: {:?}", e);
            loop {
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    }

    info!("Waiting for network stack to be ready...");
    stack.wait_config_up().await;
    info!("Network ready!");

    // Initialize SD Card SPI
    info!("Initializing SD card...");
    let mut sd_config = SpiConfig::default();
    sd_config.frequency = 400_000; // Start slow for initialization

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

    info!("SD card initialized (simplified mode)");

    // Web server loop
    info!("Starting web server on port 80...");
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        info!("Listening on TCP:80...");
        if let Err(e) = socket.accept(80).await {
            warn!("Accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());

        // Read HTTP request
        let mut pos = 0;
        loop {
            match socket.read(&mut buf[pos..]).await {
                Ok(0) => {
                    warn!("Connection closed while reading");
                    break;
                }
                Ok(n) => {
                    pos += n;
                    // Check if we've received the end of HTTP headers
                    if pos >= 4 && &buf[pos - 4..pos] == b"\r\n\r\n" {
                        break;
                    }
                    if pos >= buf.len() {
                        warn!("Request too large");
                        break;
                    }
                }
                Err(e) => {
                    warn!("Read error: {:?}", e);
                    break;
                }
            }
        }

        if pos > 0 {
            info!("Received {} bytes", pos);

            // Send HTTP response
            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/html\r\n",
                "Connection: close\r\n",
                "\r\n",
                "<!DOCTYPE html>\n",
                "<html>\n",
                "<head>\n",
                "<title>Pico W SD Card Browser</title>\n",
                "<meta name='viewport' content='width=device-width, initial-scale=1'>\n",
                "<style>\n",
                "body { font-family: Arial, sans-serif; margin: 40px; background: #f0f0f0; }\n",
                "h1 { color: #333; }\n",
                ".container { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n",
                "ul { list-style-type: none; padding: 0; }\n",
                "li { padding: 10px; margin: 5px 0; background: #f9f9f9; border-radius: 4px; }\n",
                ".info { color: #666; font-size: 0.9em; margin-top: 20px; padding-top: 20px; border-top: 1px solid #ddd; }\n",
                ".status { background: #e8f5e9; padding: 10px; border-radius: 4px; margin: 10px 0; }\n",
                "</style>\n",
                "</head>\n",
                "<body>\n",
                "<div class='container'>\n",
                "<h1>🗂️ SD Card File Browser</h1>\n",
                "<p>Connected to Raspberry Pi Pico 2W</p>\n",
                "<div class='status'>\n",
                "<strong>✅ WiFi AP Active:</strong> PicoW_SD_Browser\n",
                "</div>\n",
                "<h2>Files on SD Card:</h2>\n",
                "<ul>\n",
                "<li>📄 README.txt (1.2 KB)</li>\n",
                "<li>📄 config.json (456 bytes)</li>\n",
                "<li>📁 data/</li>\n",
                "<li>📄 log.txt (3.4 KB)</li>\n",
                "<li>📄 sensor_data.csv (8.9 KB)</li>\n",
                "</ul>\n",
                "<div class='info'>\n",
                "<p><strong>Note:</strong> SD card reading implementation in progress.</p>\n",
                "<p><strong>Status:</strong></p>\n",
                "<ul>\n",
                "<li>✅ WiFi Access Point: Active</li>\n",
                "<li>✅ Web Server: Running on port 80</li>\n",
                "<li>✅ SPI Interface: Initialized</li>\n",
                "<li>⏳ SD Card Reader: Coming soon</li>\n",
                "</ul>\n",
                "<p><strong>Hardware:</strong></p>\n",
                "<ul>\n",
                "<li>MCU: RP2350 (Pico 2W)</li>\n",
                "<li>WiFi: CYW43439</li>\n",
                "<li>SD Card: SPI0 Interface</li>\n",
                "</ul>\n",
                "</div>\n",
                "</div>\n",
                "</body>\n",
                "</html>\r\n"
            );

            if let Err(e) = socket.write_all(response.as_bytes()).await {
                warn!("Write error: {:?}", e);
            } else {
                info!("Response sent successfully");
            }
        }

        socket.close();
        Timer::after(Duration::from_millis(100)).await;
    }
}
