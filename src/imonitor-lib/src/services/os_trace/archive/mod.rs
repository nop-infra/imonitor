pub mod errors;

use errors::ArchiveError;
use plist::Value;
use std::fs::File;
use std::io::{Cursor, Read};
use std::ops::Range;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tar::Archive;

fn read_info_plist_from_tar(path: impl AsRef<Path>) -> Result<Value, ArchiveError> {
    let file = File::open(path)?;
    let mut archive = Archive::new(file);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        if path == std::path::Path::new("./Info.plist") {
            let mut buffer = Vec::new();
            entry.read_to_end(&mut buffer)?;

            let cursor = Cursor::new(&buffer);
            let plist = Value::from_reader(cursor).map_err(ArchiveError::PlistParsing)?;

            return Ok(plist);
        }
    }

    Err(ArchiveError::NoPlist)
}

pub fn extract_time_coverage_from_tar(
    path: impl AsRef<Path>,
) -> Result<Range<SystemTime>, ArchiveError> {
    let plist = read_info_plist_from_tar(path)?;

    if let Some(start_time) = extract_start_time(&plist)
        && let Some(end_time) = extract_end_time(&plist)
    {
        let start_time_st = SystemTime::UNIX_EPOCH + Duration::from_secs(start_time);
        let end_time_st = SystemTime::UNIX_EPOCH + Duration::from_secs(end_time);
        Ok(start_time_st..end_time_st)
    } else {
        Err(ArchiveError::ValueInPlist)
    }
}

fn extract_start_time(value: &Value) -> Option<u64> {
    let dict = value.as_dictionary()?;

    let live = dict.get("LiveMetadata")?.as_dictionary()?;

    let high_volume = dict.get("HighVolumeMetadata")?.as_dictionary()?;

    let sign_post = dict.get("SignPostMetadata")?.as_dictionary()?;

    let special = dict.get("SpecialMetadata")?.as_dictionary()?;

    let values = [live, high_volume, sign_post, special];

    values
        .iter()
        .filter_map(|v| {
            v.get("OldestTimeRef")?
                .as_dictionary()?
                .get("WallTime")?
                .as_unsigned_integer()
        })
        .collect::<Vec<u64>>()
        .iter()
        .min()
        .copied()
}

fn extract_end_time(value: &Value) -> Option<u64> {
    value
        .as_dictionary()?
        .get("EndTimeRef")?
        .as_dictionary()?
        .get("WallTime")?
        .as_unsigned_integer()
}
