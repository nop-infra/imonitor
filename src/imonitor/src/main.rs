use imonitor_lib::CONFIG_ENV;
use imonitor_lib::config::Config;
use imonitor_lib::device::Device;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub mod monitored_devices;
use monitored_devices::MonitoredDevices;
use tokio::sync::watch;
const MONITORED_DEVICES_FILE_PATH: &str = "devices.toml";
const CONFIG_FILE_NAME: &str = "config.toml";

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
        None => config_folder.join(CONFIG_FILE_NAME),
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

    //let mut monitor_tasks = tokio::task::JoinSet::new();

    for device_config in monitored_devices.devices {
        // Get base path from config
        let base_path;
        {
            base_path = config
                .read()
                .expect("Failed to get config read lock for base_path")
                .get_base_dir();
        }

        // Initialize device from monitored devices config
        let mut device: Device = match device_config.clone().try_into_device(base_path) {
            Ok(device) => device,
            Err(e) => {
                println!("Failed to create device from config: {e}");
                return;
            }
        };

        // Create device dirs on fs
        if let Err(e) = device.create_dirs() {
            println!("Failed to create dirs for device {}: {e}", device.info.udid);
            return;
            //continue;
        }

        // Load activity coverage from fs
        if let Err(e) = device.load_activity_coverage().await {
            println!(
                "Failed to load activity coverage for device {}: {e}",
                device.info.udid
            );
            // If it fails, we loose the recorded activity
            continue;
        }

        // Add device to vec of succeded devices to monitor and
        // write pairing file to final destination
        // This will be used to write final devices.toml file
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
        }

        if let Err(e) = device.init_logger() {
            println!("Failed to init logger for device {}: {e}", device.info.udid);
            return;
        }

        let config_clone = config.clone();
        // Add device monitor task to queue. Will be awaited
        let (tx, mut rx) = watch::channel(false);
        device.maintain_heartbeat(config_clone, &tx).await.unwrap();
        //monitor_tasks.spawn(async move { device.monitor(config_clone).await });
    }

    /*
    // Await all monitored devices tasks
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
    */
}
