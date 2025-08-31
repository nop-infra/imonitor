#[derive(Debug)]
pub enum ActivityCoverageError {
    CreateFile(std::io::Error, String),
    ReadFile(std::io::Error, String),
    FileExists(std::io::Error, String),
    WriteToFile(std::io::Error, String),
    Serialize(serde_json::Error),
    Deserialize(serde_json::Error),
}

impl std::error::Error for ActivityCoverageError {}

impl std::fmt::Display for ActivityCoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ActivityCoverageError::CreateFile(e, file_name) => {
                write!(f, "Failed to create file {file_name}: {e}")
            }
            ActivityCoverageError::ReadFile(e, file_name) => {
                write!(f, "Failed to read file {file_name}: {e}")
            }
            ActivityCoverageError::FileExists(e, file_name) => {
                write!(f, "Failed to check if file {file_name} exists: {e}")
            }
            ActivityCoverageError::WriteToFile(e, file_name) => {
                write!(f, "Failed to write to file {file_name}: {e}")
            }
            ActivityCoverageError::Serialize(e) => {
                write!(f, "Failed to serialize known crashes: {e}")
            }
            ActivityCoverageError::Deserialize(e) => {
                write!(f, "Failed to deserialize known crashes: {e}")
            }
        }
    }
}
