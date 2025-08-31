use idevice::IdeviceError;

#[derive(Debug)]
pub enum HeartbeatError {
    Timeout,
    DeviceSleeping,
    UnexpectedResponse,
    UnexpectedError(IdeviceError),
    WriteToFile(std::io::Error, String),
    CreateFile(std::io::Error, String),
    SerializeDate(serde_json::Error),
    SendConnectedState(tokio::sync::watch::error::SendError<bool>),
    ConfigReadLock,
}

impl std::error::Error for HeartbeatError {}

impl std::fmt::Display for HeartbeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HeartbeatError::Timeout => write!(f, "Heartbeat timed out"),
            HeartbeatError::DeviceSleeping => write!(f, "Device is sleeping"),
            HeartbeatError::UnexpectedResponse => write!(f, "Unexpected response from device"),
            HeartbeatError::UnexpectedError(e) => write!(f, "Unexpected error returned: {e}"),
            HeartbeatError::ConfigReadLock => write!(f, "Failed to get config read lock"),
            HeartbeatError::SendConnectedState(e) => {
                write!(f, "Failed to send connected state in channel: {e}")
            }
            HeartbeatError::WriteToFile(e, file_name) => {
                write!(f, "Failed to write to file {file_name}: {e}")
            }
            HeartbeatError::CreateFile(e, file_name) => {
                write!(f, "Failed to create file {file_name}: {e}")
            }
            HeartbeatError::SerializeDate(e) => {
                write!(f, "Failed to serialize date: {e}")
            }
        }
    }
}

impl From<IdeviceError> for HeartbeatError {
    fn from(error: IdeviceError) -> Self {
        match error {
            IdeviceError::HeartbeatTimeout => HeartbeatError::Timeout,
            IdeviceError::HeartbeatSleepyTime => HeartbeatError::DeviceSleeping,
            IdeviceError::UnexpectedResponse => HeartbeatError::UnexpectedResponse,
            e => HeartbeatError::UnexpectedError(e),
        }
    }
}
