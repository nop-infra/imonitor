use idevice::IdeviceError;

#[derive(Debug)]
pub enum DiagnosticsServiceError {
    OpenFile(std::io::Error),
    WriteToFile(std::io::Error),
    Connect(IdeviceError),
    CreateSoftwareTunnel,
    ConnectAdapterStream,
    Timeout,
    ReadLock,
    WriteLock,
}

impl std::error::Error for DiagnosticsServiceError {}

impl std::fmt::Display for DiagnosticsServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DiagnosticsServiceError::WriteToFile(e) => {
                write!(f, "Failed to write RemoteXPC services file: {e}")
            }
            DiagnosticsServiceError::OpenFile(e) => {
                write!(f, "Failed to open/create RemoteXPC services file: {e}")
            }
            DiagnosticsServiceError::Connect(e) => {
                write!(f, "Failed to connect to core device proxy service: {e}")
            }
            DiagnosticsServiceError::CreateSoftwareTunnel => {
                write!(f, "Failed to create software tunnel")
            }
            DiagnosticsServiceError::ConnectAdapterStream => {
                write!(f, "Failed to create adapter stream")
            }
            DiagnosticsServiceError::Timeout => write!(f, "CoreDeviceProxy waiting timeout"),
            DiagnosticsServiceError::ReadLock => {
                write!(f, "Failed acquiring core device proxy read lock")
            }
            DiagnosticsServiceError::WriteLock => {
                write!(f, "Failed acquiring core device proxy write lock")
            }
        }
    }
}
