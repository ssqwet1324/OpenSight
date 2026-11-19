#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

#[path = "../oled.rs"]
mod oled;
#[path = "../server_config.rs"]
mod server_config;
#[path = "../text_layout.rs"]
mod text_layout;
#[path = "../websocket.rs"]
mod websocket;

use embassy_executor::Spawner;
use embassy_net::{Runner, StackResources};
use embassy_time::{Duration, Timer, with_timeout};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    i2c::master::{Config as I2cConfig, I2c, SoftwareTimeout},
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, Interface, WifiController, sta::StationConfig,
};
use static_cell::StaticCell;

const WIFI_SSID: &str = include_str!("../../wifi-ssid.txt");
const WIFI_PASSWORD: &str = include_str!("../../wifi-password.txt");

static NETWORK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn network_task(mut runner: Runner<'static, Interface>) {
    runner.run().await;
}

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("OpenSight started!");

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    Timer::after(Duration::from_millis(100)).await;
    let oled_config = I2cConfig::default().with_software_timeout(SoftwareTimeout::Transaction(
        esp_hal::time::Duration::from_millis(250),
    ));
    let oled_display = match I2c::new(peripherals.I2C0, oled_config) {
        Ok(bus) => {
            match oled::show_startup(bus.with_sda(peripherals.GPIO4).with_scl(peripherals.GPIO5)) {
                Ok(display) => Some(display),
                Err(error) => {
                    println!("OLED: {}", error);
                    None
                }
            }
        }
        Err(error) => {
            println!("OLED I2C initialization failed: {:?}", error);
            None
        }
    };

    if let Some(display) = oled_display {
        spawner.spawn(oled::typewriter(display).expect("Failed to create display task"));
    }

    let password = WIFI_PASSWORD.trim_end_matches(['\r', '\n']);
    let ssid = WIFI_SSID.trim_end_matches(['\r', '\n']);
    if password.is_empty() || ssid.is_empty() {
        loop {
            println!("Set wifi-ssid.txt and wifi-password.txt, then rebuild and flash.");
            Timer::after(Duration::from_secs(5)).await;
        }
    }

    let mut wifi = WifiController::new(peripherals.WIFI, Default::default())
        .expect("Failed to initialize Wi-Fi");

    let station_config = StationConfig::default()
        .with_ssid(ssid.try_into().expect("Invalid Wi-Fi SSID length"))
        .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
            password.try_into().expect("Invalid Wi-Fi password length"),
        ));

    wifi.set_config(&Config::Station(station_config))
        .expect("Failed to configure Wi-Fi");

    let rng = Rng::new();
    let seed = ((rng.random() as u64) << 32) | rng.random() as u64;
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        NETWORK_RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(network_task(runner).expect("Failed to create network task"));
    spawner.spawn(websocket::receiver(stack).expect("Failed to create WebSocket task"));

    loop {
        if !wifi.is_connected() {
            println!("Connecting to {}...", ssid);

            match wifi.connect_async().await {
                Ok(_) => println!("Wi-Fi connected"),
                Err(error) => {
                    println!("Wi-Fi connection failed: {:?}", error);
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            }
        }

        if stack.config_v4().is_none() {
            println!("Waiting for an IP address (DHCP)...");
            if with_timeout(Duration::from_secs(30), stack.wait_config_up())
                .await
                .is_err()
            {
                println!("DHCP timeout; retrying...");
            }
        }

        if let Some(config) = stack.config_v4() {
            println!("OpenSight is alive | IP: {}", config.address.address());
        }

        Timer::after(Duration::from_secs(10)).await;
    }
}
