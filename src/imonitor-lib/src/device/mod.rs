pub mod activity_coverage;
pub mod errors;

use crate::config::Config;
use activity_coverage::ACTIVITY_COVERAGE_FILE_NAME;
use activity_coverage::ActivityCoverage;
use chrono::{DateTime, Utc};
use errors::DeviceError;
use idevice::pairing_file::PairingFile;
use idevice::provider::{IdeviceProvider, TcpProvider};
use logger::{HasLogger, Logger};
use phf::phf_map;
use std::collections::HashSet;
use std::fs::create_dir_all;
use std::net::IpAddr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;
use tokio::{task::JoinHandle, try_join};

static SUB_DIRS: phf::Map<&'static str, &'static str> = phf_map! {
    "info" => "info",
    "connection" => "connection",
    "heartbeat" => "heartbeat",
    "crashes" => "crashes",
    "crash_files" => "crashes/files",
    "syslog" => "syslog",
    "os_trace" => "os_trace",
    "os_trace_log" => "os_trace/log",
    "os_trace_archive" => "os_trace/archive",
    "os_trace_pid" => "os_trace/pid",
    "activity_coverage" => "activity_coverage",
};

#[derive(Debug, Clone)]
pub struct Device {
    pub info: Info,
    pub connection: Connection,
    pub heartbeat: HeartBeat,
    pub crashes: Crashes,
    pub logger: Option<Arc<Logger>>,
    pub activity_coverage: Arc<RwLock<ActivityCoverage>>,
    pub base_dir: String,
}

#[derive(Debug, Clone)]
pub struct Info {
    pub udid: String,
}

#[derive(Debug, Clone)]
pub struct Crashes {
    pub crash_files: Arc<RwLock<HashSet<String>>>,
    pub crash_dirs: Arc<RwLock<HashSet<String>>>,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub ip_addr: IpAddr,
    pub pairing_file: PairingFile,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct HeartBeat {
    pub last_established: Arc<RwLock<DateTime<Utc>>>,
}

impl Info {
    pub fn new(udid: &str) -> Info {
        Info {
            udid: udid.to_string(),
        }
    }
}

impl Crashes {
    pub fn new() -> Crashes {
        Crashes {
            crash_files: Arc::new(RwLock::new(HashSet::new())),
            crash_dirs: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl Default for Crashes {
    fn default() -> Self {
        Self::new()
    }
}

impl HeartBeat {
    pub fn new() -> HeartBeat {
        HeartBeat {
            last_established: Arc::new(RwLock::new(DateTime::<Utc>::MIN_UTC)),
        }
    }
}

impl Default for HeartBeat {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new(pairing_file: &PairingFile, ip_addr: &IpAddr, label: &str) -> Connection {
        Connection {
            ip_addr: *ip_addr,
            pairing_file: pairing_file.clone(),
            label: label.to_string(),
        }
    }
}

impl Device {
    pub fn new(
        udid: &str,
        pairing_file: &PairingFile,
        ip_addr: &IpAddr,
        label: &str,
        base_dir: impl AsRef<Path>,
    ) -> Device {
        let connection = Connection::new(pairing_file, ip_addr, label);

        Device {
            connection: connection.clone(),
            info: Info::new(udid),
            heartbeat: HeartBeat::new(),
            crashes: Crashes::new(),
            logger: None,
            activity_coverage: Arc::new(RwLock::new(ActivityCoverage::new())),
            base_dir: base_dir.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn load_activity_coverage(&mut self) -> Result<(), DeviceError> {
        let activity_coverage =
            activity_coverage::load_from_fs(&self.get_activity_coverage_file_path()).await?;
        self.activity_coverage = Arc::new(RwLock::new(activity_coverage));
        Ok(())
    }

    pub fn base_dir(&self) -> String {
        let general_base_dir = PathBuf::from(&self.base_dir);
        general_base_dir
            .join(self.info.udid.clone())
            .to_string_lossy()
            .to_string()
    }

    pub fn get_heartbeat_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("heartbeat").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_syslog_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("syslog").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_pairing_file_name(&self) -> String {
        let mut udid = self.info.udid.clone();
        udid.push_str(".plist");
        udid
    }

    pub fn get_pairing_file_path(&self) -> String {
        let connection_dir = PathBuf::from(self.get_connection_dir());
        connection_dir
            .join(self.get_pairing_file_name())
            .to_string_lossy()
            .to_string()
    }

    pub fn get_connection_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("connection").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_crashes_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("crashes").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_crash_files_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("crash_files").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_log_file_name(&self) -> String {
        format!("{}.log", self.info.udid)
    }

    pub fn get_os_trace_log_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("os_trace_log").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_os_trace_archive_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("os_trace_archive").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_activity_coverage_dir(&self) -> String {
        let base_path = PathBuf::from(self.base_dir());
        base_path
            .join(SUB_DIRS.get("activity_coverage").unwrap_or(&""))
            .to_string_lossy()
            .to_string()
    }

    pub fn get_activity_coverage_file_path(&self) -> String {
        let dir = PathBuf::from(self.get_activity_coverage_dir());
        dir.join(ACTIVITY_COVERAGE_FILE_NAME)
            .to_string_lossy()
            .to_string()
    }

    pub fn create_dirs(&self) -> Result<(), DeviceError> {
        for dir in SUB_DIRS.values() {
            let base_path = PathBuf::from(self.base_dir());
            let path = base_path.join(dir).to_string_lossy().to_string();
            create_dir_all(&path).map_err(|e| DeviceError::CreateDir(e, path.clone()))?;
        }

        Ok(())
    }

    pub async fn write_pairing_file(&self, _source_file_path: &str) -> Result<(), DeviceError> {
        let pairing_file_bytes = self
            .connection
            .pairing_file
            .clone()
            .serialize()
            .map_err(DeviceError::SerializePairingFile)?;

        let pairing_file_path = self.get_pairing_file_path();

        let file_h = File::create(pairing_file_path.clone())
            .await
            .map_err(|e| DeviceError::CreateFile(e, pairing_file_path.clone()))?;

        let mut writer = BufWriter::new(file_h);

        writer
            .write_all(&pairing_file_bytes)
            .await
            .map_err(|e| DeviceError::WriteToFile(e, pairing_file_path.clone()))?;

        writer
            .flush()
            .await
            .map_err(|e| DeviceError::WriteToFile(e, pairing_file_path.clone()))?;

        /*
        if source_file_path != pairing_file_path {
            remove_file(source_file_path)
                .await
                .map_err(|e| DeviceError::RemoveFile(e, pairing_file_path.clone()))?;
        }
        */

        Ok(())
    }

    pub async fn monitor(&mut self, config: Arc<RwLock<Config>>) -> Result<(), DeviceError> {
        let refresh_rate;
        {
            refresh_rate = config
                .read()
                .map_err(|_| DeviceError::ConfigReadLock)?
                .settings
                .clone()
                .refresh_rate;
        }

        let (tx, mut rx) = watch::channel(false);

        let device_hb = self.clone();
        //let device_syslog = self.clone();
        let device_crashes = self.clone();
        let device_os_trace_log = self.clone();
        let device_os_trace_archive = self.clone();

        //let mut syslog_hb_rx = rx.clone();
        let mut os_trace_log_hb_rx = rx.clone();
        let mut os_trace_archive_hb_rx = rx.clone();

        /*
        // No parallelized version
        let hb = self.maintain_heartbeat(config);
        let syslog = self.stream_syslog(refresh_rate);
        let crashes = self.get_crashes(refresh_rate);
        let _ = tokio::join!(hb, syslog, crashes);
        */

        let hb = tokio::spawn(async move { device_hb.maintain_heartbeat(config, &tx).await });
        /*
        let _syslog =
            tokio::spawn(async move { device_syslog.stream_syslog(refresh_rate, &mut syslog_hb_rx).await });
        */
        let crashes =
            tokio::spawn(async move { device_crashes.get_crashes(refresh_rate, &mut rx).await });
        let os_trace_log = tokio::spawn(async move {
            device_os_trace_log
                .stream_os_trace_logs(refresh_rate, &mut os_trace_log_hb_rx)
                .await
        });
        let os_trace_archive = tokio::spawn(async move {
            device_os_trace_archive
                .create_os_trace_archive(refresh_rate, &mut os_trace_archive_hb_rx)
                .await
        });

        /*
        let _ = hb.await;
        let _ = syslog.await;
        let _ = crashes.await;
        */

        try_join!(
            flatten(hb),
            //flatten(syslog),
            flatten(crashes),
            flatten(os_trace_log),
            //flatten(os_trace_archive),
        )?;

        Ok(())
    }

    pub fn init_logger(&mut self) -> Result<(), DeviceError> {
        let logger = Logger::new(&self.base_dir(), &self.get_log_file_name());
        self.logger = Some(Arc::new(logger));
        Ok(())
    }
}

// Inspired from https://docs.rs/tokio/latest/tokio/macro.try_join.html
async fn flatten(
    handle: JoinHandle<Result<(), impl Into<DeviceError>>>,
) -> Result<(), DeviceError> {
    match handle.await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(err.into()),
    }
}

impl HasLogger for Device {
    fn logger(&self) -> Option<&Logger> {
        self.logger.as_deref()
    }
}

impl From<&Device> for TcpProvider {
    fn from(device: &Device) -> Self {
        TcpProvider {
            addr: device.connection.ip_addr,
            pairing_file: device.connection.pairing_file.clone(),
            label: device.connection.label.clone(),
        }
    }
}

impl From<&Device> for Box<dyn IdeviceProvider> {
    fn from(device: &Device) -> Self {
        let provider: TcpProvider = device.into();
        Box::new(provider)
    }
}
