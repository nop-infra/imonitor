use idevice::IdeviceError;

#[derive(Debug)]
pub enum RemoteXPCError {
    OpenFile(std::io::Error),
    WriteToFile(std::io::Error),
    Connect(IdeviceError),
    CreateSoftwareTunnel,
    ConnectAdapterStream,
    Timeout,
    ReadLock,
    WriteLock,
}

impl std::error::Error for RemoteXPCError {}

impl std::fmt::Display for RemoteXPCError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RemoteXPCError::WriteToFile(e) => {
                write!(f, "Failed to write RemoteXPC services file: {e}")
            }
            RemoteXPCError::OpenFile(e) => {
                write!(f, "Failed to open/create RemoteXPC services file: {e}")
            }
            RemoteXPCError::Connect(e) => {
                write!(f, "Failed to connect to core device proxy service: {e}")
            }
            RemoteXPCError::CreateSoftwareTunnel => write!(f, "Failed to create software tunnel"),
            RemoteXPCError::ConnectAdapterStream => write!(f, "Failed to create adapter stream"),
            RemoteXPCError::Timeout => write!(f, "CoreDeviceProxy waiting timeout"),
            RemoteXPCError::ReadLock => write!(f, "Failed acquiring core device proxy read lock"),
            RemoteXPCError::WriteLock => write!(f, "Failed acquiring core device proxy write lock"),
        }
    }
}
