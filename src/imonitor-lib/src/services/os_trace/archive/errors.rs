#[derive(Debug)]
pub enum ArchiveError {
    IO(std::io::Error),
    PlistParsing(plist::Error),
    NoPlist,
    ValueInPlist,
}

impl std::error::Error for ArchiveError {}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ArchiveError::IO(e) => write!(f, "IO error while reading archive: {e}"),
            ArchiveError::PlistParsing(e) => write!(f, "Failed to read Info.plist: {e}"),
            ArchiveError::NoPlist => write!(f, "Failed to retrieve Info.plist in archive"),
            ArchiveError::ValueInPlist => {
                write!(f, "Failed to retrieve start/end time in archive plist")
            }
        }
    }
}

impl From<std::io::Error> for ArchiveError {
    fn from(error: std::io::Error) -> Self {
        ArchiveError::IO(error)
    }
}
