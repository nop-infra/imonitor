use idevice::pairing_file::PairingFile;
use imonitor_lib::device::Device;
use imonitor_lib::device::errors::DeviceError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::fs::read_to_string;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

fn default_connection_label() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceConfig {
    pub udid: String,
    pub pairing_file_path: String,
    pub ip: std::net::IpAddr,
    #[serde(default = "default_connection_label")]
    pub connection_label: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MonitoredDevices {
    pub devices: Vec<DeviceConfig>,
}

impl MonitoredDevices {
    /// Parses the config file and returns the values.
    pub fn parse(path: &Path) -> Result<MonitoredDevices, Box<dyn Error>> {
        let devices_str = read_to_string(path)?;
        let devices: MonitoredDevices = toml::from_str(&devices_str)?;
        Ok(devices)
    }

    pub fn write_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let monitored_devices = toml::to_string(&self)?;

        let file_h = File::create(path)?;

        let mut writer = BufWriter::new(file_h);

        writer.write_all(&monitored_devices.into_bytes())?;

        writer.flush()?;

        Ok(())
    }
}

impl DeviceConfig {
    pub fn try_into_device(self, base_dir: impl AsRef<Path>) -> Result<Device, DeviceError> {
        let pairing_file = PairingFile::read_from_file(&self.pairing_file_path)
            .map_err(DeviceError::ReadPairingFile)?;

        Ok(Device::new(
            &self.udid,
            &pairing_file,
            &self.ip,
            &self.connection_label,
            base_dir.as_ref(),
        ))
    }
}

/*
impl TryInto<Device> for DeviceConfig {
    type Error = DeviceError;

    fn try_into(self) -> Result<Device, DeviceError> {
        let pairing_file = PairingFile::read_from_file(&self.pairing_file_path)
            .map_err(DeviceError::ReadPairingFile)?;

        Ok(Device::new(
            &self.udid,
            &pairing_file,
            &self.ip,
            &self.connection_label,
        ))
    }
}
*/
