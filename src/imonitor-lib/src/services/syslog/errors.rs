use idevice::IdeviceError;

#[derive(Debug)]
pub enum SyslogError {
    OpenFile(std::io::Error),
    WriteToFile(std::io::Error),
    Connect(IdeviceError),
    HeartbeatWatch(tokio::sync::watch::error::RecvError),
    Timeout,
}

impl std::error::Error for SyslogError {}

impl std::fmt::Display for SyslogError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SyslogError::WriteToFile(e) => write!(f, "Failed to write to syslog file : {e}"),
            SyslogError::OpenFile(e) => write!(f, "Failed to open/create syslog file : {e}"),
            SyslogError::Connect(e) => write!(f, "Failed to connect to syslog service : {e}"),
            SyslogError::HeartbeatWatch(e) => write!(f, "Heartbeat watch receiver failed: {e}"),
            SyslogError::Timeout => write!(f, "Syslog waiting timeout"),
        }
    }
}

impl From<IdeviceError> for SyslogError {
    fn from(error: IdeviceError) -> Self {
        SyslogError::Connect(error)
    }
}
