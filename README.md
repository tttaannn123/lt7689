# LT7689 - Raspberry Pi Pico 2W SD Card Browser

A WiFi-enabled SD card file browser running on the Raspberry Pi Pico 2W, built with Rust and Embassy.

## Features

- 🔌 **WiFi Access Point** - Creates its own WiFi network for easy access
- 🗂️ **SD Card Interface** - SPI-based SD card reader (implementation in progress)
- 🌐 **Web Server** - Browse SD card contents via web browser
- ⚡ **Async/Await** - Built with Embassy for efficient async operations
- 🦀 **Rust** - Memory-safe embedded development

## Hardware Requirements

- Raspberry Pi Pico 2W (RP2350 with CYW43439 WiFi)
- MicroSD card module connected via SPI
- USB cable for power and programming

## Wiring

### SD Card Module (SPI0)
- **SCK/CLK** → GPIO 18
- **MOSI** → GPIO 19
- **MISO** → GPIO 16
- **CS** → GPIO 17
- **VCC** → 3.3V
- **GND** → GND

## Building

### Prerequisites

1. Install Rust (rustup):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Add ARM Cortex-M target:
```bash
rustup target add thumbv8m.main-none-eabihf
```

3. Install probe-rs (for flashing):
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-installer.sh | sh
```

### Build Firmware

```bash
cargo build --release --target thumbv8m.main-none-eabihf
```

### Flash to Pico

```bash
cargo run --release
```

Or manually copy the UF2 file to the Pico in bootloader mode.

## Usage

1. Flash the firmware to your Pico 2W
2. The device will create a WiFi access point named **`PicoW_SD_Browser`**
3. Connect to this WiFi network using password: **`12345678`**
4. Open your web browser and navigate to: **`http://192.168.4.1`**
5. View the SD card contents in your browser

## WiFi Credentials

- **SSID**: `PicoW_SD_Browser`
- **Password**: `12345678`
- **IP Address**: `192.168.4.1`
- **Port**: `80` (HTTP)

## Configuration

To change WiFi credentials, edit `src/main.rs`:

```rust
const WIFI_NETWORK: &str = "PicoW_SD_Browser";
const WIFI_PASSWORD: &str = "12345678";
```

## Project Structure

```
lt7689/
├── src/
│   └── main.rs          # Main application code
├── cyw43-firmware/      # WiFi firmware files
│   ├── 43439A0.bin
│   └── 43439A0_clm.bin
├── .cargo/
│   └── config.toml      # Cargo build configuration
├── build.rs             # Build script
├── memory.x             # Memory layout
├── Cargo.toml           # Dependencies
└── README.md            # This file
```

## Development Status

- ✅ WiFi Access Point functionality
- ✅ Web server on port 80
- ✅ SPI interface initialization
- ⏳ SD card FAT filesystem reading (in progress)
- ⏳ File browsing interface
- ⏳ File download capability

## Dependencies

This project uses:
- **Embassy** - Async embedded framework
- **cyw43** - WiFi driver for CYW43439
- **embassy-net** - TCP/IP networking stack
- **embedded-sdmmc** - FAT filesystem implementation
- **defmt** - Efficient logging for embedded systems

## Troubleshooting

### Build Errors

If you encounter build errors:
1. Make sure you have the correct Rust toolchain: `rustup update`
2. Clean the build cache: `cargo clean`
3. Rebuild: `cargo build --release`

### Cannot Connect to WiFi

1. Verify the Pico is powered on (LED should be blinking/active)
2. Check that your device supports 2.4GHz WiFi (5GHz is not supported)
3. Try forgetting and reconnecting to the network
4. Check serial output with `probe-rs` for error messages

**Note:** Some devices may show the network but fail to connect on first try. Wait a few seconds and try again.

### SD Card Not Detected

**Important:** Make sure you're connecting to the correct pins!
- **SCK** (on SD module) connects to **GPIO 18**
- **MOSI** connects to **GPIO 19**
- **MISO** connects to **GPIO 16**
- **CS** connects to **GPIO 17**

If still not working:
1. Verify wiring connections - double-check each wire
2. Ensure SD card is formatted as **FAT32** (not exFAT or NTFS)
3. Try a smaller SD card (≤32GB works best)
4. Check that SD card is fully inserted in the module
5. Verify 3.3V power is connected (not 5V!)
6. Try a different SD card (some older cards may not be compatible)

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Resources

- [Embassy Documentation](https://embassy.dev/)
- [RP2350 Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- [Pico 2W Datasheet](https://datasheets.raspberrypi.com/picow/pico-2-w-datasheet.pdf)
- [Rust Embedded Book](https://docs.rust-embedded.org/book/)