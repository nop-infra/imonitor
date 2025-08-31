use super::archive::errors::ArchiveError;
use crate::device::activity_coverage::errors::ActivityCoverageError;
use idevice::IdeviceError;

#[derive(Debug)]
pub enum OsTraceError {
    OpenFile(std::io::Error),
    WriteToFile(std::io::Error),
    Connect(IdeviceError),
    CreateArchive(IdeviceError),
    HeartbeatWatch(tokio::sync::watch::error::RecvError),
    SerializeLog(serde_json::Error),
    OppositeTime(std::time::SystemTimeError),
    ActivityCoverage(ActivityCoverageError),
    Archive(ArchiveError),
    Timeout,
    ReadLock,
    WriteLock,
}

impl std::error::Error for OsTraceError {}

impl std::fmt::Display for OsTraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OsTraceError::WriteToFile(e) => write!(f, "Failed to write to os trace log file: {e}"),
            OsTraceError::OpenFile(e) => write!(f, "Failed to open/create os trace log file: {e}"),
            OsTraceError::Connect(e) => {
                write!(f, "Failed to connect to os trace log service: {e}")
            }
            OsTraceError::CreateArchive(e) => {
                write!(f, "Failed to create os trace archive: {e}")
            }
            OsTraceError::HeartbeatWatch(e) => write!(f, "Heartbeat watch receiver failed: {e}"),
            OsTraceError::SerializeLog(e) => write!(f, "Failed to serialize log: {e}"),
            OsTraceError::ActivityCoverage(e) => write!(f, "ActivityCoverageError: {e}"),
            OsTraceError::OppositeTime(e) => {
                write!(f, "Opposite time, difference is: {:?}", e.duration())
            }
            OsTraceError::Timeout => write!(f, "OsTrace waiting timeout"),
            OsTraceError::ReadLock => write!(f, "Failed acquiring os trace read lock"),
            OsTraceError::WriteLock => write!(f, "Failed acquiring os trace write lock"),
            OsTraceError::Archive(e) => write!(f, "Failed processing archive: {e}"),
        }
    }
}

impl From<ActivityCoverageError> for OsTraceError {
    fn from(error: ActivityCoverageError) -> Self {
        OsTraceError::ActivityCoverage(error)
    }
}

impl From<ArchiveError> for OsTraceError {
    fn from(error: ArchiveError) -> Self {
        OsTraceError::Archive(error)
    }
}
