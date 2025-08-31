use super::activity_coverage::errors::ActivityCoverageError;
use crate::services::crashes::errors::CrashError;
use crate::services::heartbeat::errors::HeartbeatError;
use crate::services::os_trace::errors::OsTraceError;
use crate::services::syslog::errors::SyslogError;
use idevice::IdeviceError;

#[derive(Debug)]
pub enum DeviceError {
    ParseIp,
    ReadPairingFile(IdeviceError),
    SerializePairingFile(IdeviceError),
    UnexpectedError(IdeviceError),
    Heartbeat(HeartbeatError),
    WriteToFile(std::io::Error, String),
    RemoveFile(std::io::Error, String),
    Syslog(SyslogError),
    Crash(CrashError),
    OsTrace(OsTraceError),
    CreateDir(std::io::Error, String),
    CreateFile(std::io::Error, String),
    Task(tokio::task::JoinError),
    ActivityCoverage(ActivityCoverageError),
    TaskFailed,
    ConfigReadLock,
}

impl std::error::Error for DeviceError {}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeviceError::ParseIp => write!(f, "Failed to parse device ip"),
            DeviceError::ReadPairingFile(e) => write!(f, "Failed to read pairing file: {e}"),
            DeviceError::SerializePairingFile(e) => {
                write!(f, "Failed to serialize pairing file: {e}")
            }
            DeviceError::UnexpectedError(e) => write!(f, "Unexpected error returned: {e}"),
            DeviceError::Heartbeat(e) => write!(f, "Heartbeat failed: {e}"),
            DeviceError::WriteToFile(e, file_name) => {
                write!(f, "Failed to write to file {file_name}: {e}")
            }
            DeviceError::RemoveFile(e, file_name) => {
                write!(f, "Failed to remove file {file_name}: {e}")
            }
            DeviceError::Syslog(e) => write!(f, "Syslog task failed: {e}"),
            DeviceError::Crash(e) => write!(f, "Crash task failed: {e}"),
            DeviceError::OsTrace(e) => write!(f, "Os trace failed: {e}"),
            DeviceError::Task(e) => write!(f, "Tokio task failed: {e}"),
            DeviceError::ActivityCoverage(e) => write!(f, "Activity coverage error: {e}"),
            DeviceError::CreateDir(e, dir_name) => {
                write!(f, "Failed to create directory {dir_name}: {e}")
            }
            DeviceError::CreateFile(e, file_name) => {
                write!(f, "Failed to create file {file_name}: {e}")
            }
            DeviceError::ConfigReadLock => write!(f, "Failed to get config read lock"),
            DeviceError::TaskFailed => write!(f, "Spawned task failed"),
        }
    }
}

impl From<HeartbeatError> for DeviceError {
    fn from(error: HeartbeatError) -> Self {
        DeviceError::Heartbeat(error)
    }
}

impl From<SyslogError> for DeviceError {
    fn from(error: SyslogError) -> Self {
        DeviceError::Syslog(error)
    }
}

impl From<CrashError> for DeviceError {
    fn from(error: CrashError) -> Self {
        DeviceError::Crash(error)
    }
}

impl From<OsTraceError> for DeviceError {
    fn from(error: OsTraceError) -> Self {
        DeviceError::OsTrace(error)
    }
}

impl From<tokio::task::JoinError> for DeviceError {
    fn from(error: tokio::task::JoinError) -> Self {
        DeviceError::Task(error)
    }
}

impl From<ActivityCoverageError> for DeviceError {
    fn from(error: ActivityCoverageError) -> Self {
        DeviceError::ActivityCoverage(error)
    }
}

impl From<IdeviceError> for DeviceError {
    fn from(error: IdeviceError) -> Self {
        match error {
            IdeviceError::UnexpectedResponse => DeviceError::ReadPairingFile(error),
            e => DeviceError::UnexpectedError(e),
        }
    }
}
