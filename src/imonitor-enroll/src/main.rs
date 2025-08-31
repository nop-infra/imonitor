use clap::{Arg, Command};
use idevice::{
    IdeviceService,
    lockdown::LockdownClient,
    usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection},
};
use log::info;

const CONNECTION_LABEL: &str = "test";

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("pair")
        .about("Pair with the device")
        .arg(
            Arg::new("udid")
                .value_name("UDID")
                .help("UDID of the device (overrides host/pairing file)")
                .index(1),
        )
        .get_matches();

    let udid = matches.get_one::<String>("udid");

    let mut u = UsbmuxdConnection::default()
        .await
        .expect("Failed to connect to usbmuxd");

    let dev = match udid {
        Some(udid) => u
            .get_device(udid)
            .await
            .expect("Failed to get device with specific udid"),
        None => u
            .get_devices()
            .await
            .expect("Failed to get devices")
            .into_iter()
            .find(|x| x.connection_type == Connection::Usb)
            .expect("No devices connected via USB"),
    };

    let provider = dev.to_provider(UsbmuxdAddr::default(), CONNECTION_LABEL);

    let mut lockdown_client = match LockdownClient::connect(&provider).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Unable to connect to lockdown: {e:?}");
            return;
        }
    };
    let id = uuid::Uuid::new_v4().to_string().to_uppercase();

    let mut pairing_file = lockdown_client
        .pair(id, u.get_buid().await.unwrap())
        .await
        .expect("Failed to pair");

    // Test the pairing file
    lockdown_client
        .start_session(&pairing_file)
        .await
        .expect("Pairing file test failed");

    info!("Enabling lockdownd wifi connection");

    lockdown_client
        .set_value(
            "EnableWifiConnections",
            true.into(),
            Some("com.apple.mobile.wireless_lockdown"),
        )
        .await
        .expect("Failed to enable lockdownd wifi connection");

    // Add the UDID
    pairing_file.udid = Some(dev.udid.clone());

    tokio::fs::write(
        format!("{}_pairing_file.plist", dev.udid),
        pairing_file
            .serialize()
            .expect("Failed to serialize pairing file"),
    )
    .await
    .expect("Failed to write pairing file");
}
