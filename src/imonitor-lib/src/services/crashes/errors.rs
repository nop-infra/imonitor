use idevice::IdeviceError;

#[derive(Debug)]
pub enum CrashError {
    CreateDir(std::io::Error, String),
    CreateFile(std::io::Error, String),
    ReadFile(std::io::Error, String),
    FileExists(std::io::Error, String),
    WriteToFile(std::io::Error, String),
    Connect(IdeviceError),
    ListFiles(IdeviceError, String),
    PullFile(IdeviceError, String),
    SerializeKnownCrashes(serde_json::Error),
    DeserializeKnownCrashes(serde_json::Error),
    ReadLock,
    WriteLock,
    Timeout,
}

impl std::error::Error for CrashError {}

impl std::fmt::Display for CrashError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CrashError::CreateDir(e, dir_name) => {
                write!(f, "Failed to create directory {dir_name}: {e}")
            }
            CrashError::CreateFile(e, file_name) => {
                write!(f, "Failed to create file {file_name}: {e}")
            }
            CrashError::ReadFile(e, file_name) => {
                write!(f, "Failed to read file {file_name}: {e}")
            }
            CrashError::FileExists(e, file_name) => {
                write!(f, "Failed to check if file {file_name} exists: {e}")
            }
            CrashError::WriteToFile(e, file_name) => {
                write!(f, "Failed to write to file {file_name}: {e}")
            }
            CrashError::Connect(e) => write!(f, "Failed to connect to crash service: {e}"),
            CrashError::ListFiles(e, path) => {
                write!(f, "Failed to list files from path \"{path}\": {e}")
            }
            CrashError::PullFile(e, file) => write!(f, "Failed to pull file {file}: {e}"),
            CrashError::SerializeKnownCrashes(e) => {
                write!(f, "Failed to serialize known crashes: {e}")
            }
            CrashError::DeserializeKnownCrashes(e) => {
                write!(f, "Failed to deserialize known crashes: {e}")
            }
            CrashError::Timeout => write!(f, "Crash service waiting timeout"),
            CrashError::ReadLock => write!(f, "Failed acquiring crash files read lock"),
            CrashError::WriteLock => write!(f, "Failed acquiring crash files write lock"),
        }
    }
}
