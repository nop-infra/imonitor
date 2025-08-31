use imonitor_lib::CONFIG_ENV;
use imonitor_lib::config::Config;
use imonitor_lib::device::Device;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub mod monitored_devices;
use monitored_devices::MonitoredDevices;

const MONITORED_DEVICES_FILE_PATH: &str = "devices.toml";

/// Setup config
fn setup(config_folder: &Path) -> Arc<RwLock<Config>> {
    // Parse configuration.
    let config_path = match env::var(CONFIG_ENV).ok() {
        Some(path) => {
            unsafe {
                env::remove_var(CONFIG_ENV);
            }
            PathBuf::from(path)
        }
        None => config_folder.join("config.toml"),
    };

    if !config_path.exists() {
        println!(
            "{} is not found, please provide a configuration file.",
            config_path.display()
        );
        std::process::exit(1);
    }

    // Parse config
    let config = Config::parse(&config_path).expect("failed to parse config");

    // Main configuration
    Arc::new(RwLock::new(config))
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = setup(&PathBuf::new());

    let monitored_devices = MonitoredDevices::parse(&PathBuf::from(MONITORED_DEVICES_FILE_PATH))
        .expect("Failed to parse monitored devices list");

    let mut monitored_devices_final = MonitoredDevices::default();

    let mut monitor_tasks = tokio::task::JoinSet::new();

    for device_config in monitored_devices.devices {
        let mut device: Device = match device_config.clone().try_into() {
            Ok(device) => device,
            Err(e) => {
                println!("Failed to create device from config: {e}");
                return;
            }
        };
        if let Err(e) = device.create_dirs() {
            println!("Failed to create dirs for device {}: {e}", device.info.udid);
            return;
            //continue;
        }

        if let Err(e) = device.load_activity_coverage().await {
            println!(
                "Failed to load activity coverage for device {}: {e}",
                device.info.udid
            );
            // If it fails, we loose the recorded activity
            continue;
        }

        let mut device_config_final = device_config.clone();
        device_config_final.pairing_file_path = device.get_pairing_file_path();
        monitored_devices_final.devices.push(device_config_final);
        monitored_devices_final
            .write_to_file(&MONITORED_DEVICES_FILE_PATH.into())
            .expect("Failed to write to monitored devices");

        if let Err(e) = device
            .write_pairing_file(&device_config.clone().pairing_file_path)
            .await
        {
            println!(
                "Failed to write pairing file for device {}: {e}",
                device.info.udid
            );
            return;
            //continue;
        }

        if let Err(e) = device.init_logger() {
            println!("Failed to init logger for device {}: {e}", device.info.udid);
            return;
        }

        let config_clone = config.clone();
        monitor_tasks.spawn(async move { device.monitor(config_clone).await });
    }

    while let Some(res) = monitor_tasks.join_next().await {
        match res {
            Err(e) => {
                println!("Device monitoring task error: {e}");
            }
            Ok(Err(e)) => {
                println!("Device monitoring error: {e}");
            }
            Ok(Ok(_)) => {
                println!("Task finished");
            }
        }
    }
}
