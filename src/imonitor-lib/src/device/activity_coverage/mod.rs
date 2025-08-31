pub mod errors;

use chrono::{DateTime, Utc};
use errors::ActivityCoverageError;
use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ops::Range,
    path::Path,
    time::{Duration, SystemTime},
};
use tokio::fs::{File, read_to_string, try_exists};
use tokio::io::{AsyncWriteExt, BufWriter};

pub const ACTIVITY_COVERAGE_FILE_NAME: &str = "activity_coverage.json";

#[derive(Debug, Clone, Eq)]
pub struct TimeRange(Range<SystemTime>);

impl PartialEq for TimeRange {
    fn eq(&self, other: &Self) -> bool {
        self.0.start == other.0.start && self.0.end == other.0.end
    }
}

impl PartialOrd for TimeRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        //Some(self.0.start.cmp(&other.0.start))
        Some(self.cmp(other))
    }
}

impl Ord for TimeRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.start.cmp(&other.0.start)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityCoverage {
    covered: BTreeSet<TimeRange>,
}

impl Default for ActivityCoverage {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityCoverage {
    pub fn new() -> Self {
        Self {
            covered: BTreeSet::new(),
        }
    }

    pub fn add_range(&mut self, new_range: Range<SystemTime>) {
        let mut new_start = new_range.start;
        let mut new_end = new_range.end;

        // Trouver les plages existantes qui chevauchent ou touchent celle à insérer
        let mut to_remove = vec![];

        for existing in &self.covered {
            if new_end < existing.0.start || new_start > existing.0.end {
                continue;
            }
            // Fusionner
            new_start = new_start.min(existing.0.start);
            new_end = new_end.max(existing.0.end);
            to_remove.push(existing.clone());
        }

        // Supprimer les anciennes plages fusionnées
        for r in to_remove {
            self.covered.remove(&r);
        }

        // Insérer la nouvelle plage fusionnée
        self.covered.insert(TimeRange(new_start..new_end));
    }

    pub fn missing_ranges(&self) -> Vec<Range<SystemTime>> {
        let mut result = vec![];
        let mut cursor = match self.covered.first() {
            Some(time_range) => time_range.0.clone().start,
            None => return vec![],
        };

        let last_range = match self.covered.last() {
            Some(time_range) => time_range.0.clone(),
            None => return vec![],
        };

        for r in &self.covered {
            if r.0.start > cursor {
                result.push(cursor..r.0.start);
            }
            cursor = cursor.max(r.0.end);
        }

        if cursor < last_range.end {
            result.push(cursor..last_range.end);
        }

        result
    }

    pub fn oldest_gap(&self) -> Option<Range<SystemTime>> {
        self.missing_ranges().into_iter().next()
    }

    pub async fn write_to_fs(
        &self,
        output_path: impl AsRef<Path>,
    ) -> Result<(), ActivityCoverageError> {
        let coverage =
            serde_json::to_string_pretty(&self).map_err(ActivityCoverageError::Serialize)?;
        let output_path_string = output_path.as_ref().to_string_lossy().to_string();

        let file_h = File::create(output_path)
            .await
            .map_err(|e| ActivityCoverageError::CreateFile(e, output_path_string.clone()))?;

        let mut writer = BufWriter::new(file_h);

        writer
            .write_all(coverage.as_bytes())
            .await
            .map_err(|e| ActivityCoverageError::WriteToFile(e, output_path_string.clone()))?;

        writer
            .flush()
            .await
            .map_err(|e| ActivityCoverageError::WriteToFile(e, output_path_string.clone()))?;
        Ok(())
    }
}

pub async fn load_from_fs(
    path: impl AsRef<Path>,
) -> Result<ActivityCoverage, ActivityCoverageError> {
    let path_string = path.as_ref().to_string_lossy().to_string();
    if try_exists(&path)
        .await
        .map_err(|e| ActivityCoverageError::FileExists(e, path_string.clone()))?
    {
        let content = read_to_string(path_string.clone())
            .await
            .map_err(|e| ActivityCoverageError::ReadFile(e, path_string.clone()))?;

        let coverage: ActivityCoverage =
            serde_json::from_str(&content).map_err(ActivityCoverageError::Deserialize)?;

        Ok(coverage)
    } else {
        Ok(ActivityCoverage::default())
    }
}

impl TimeRange {
    fn to_rfc3339_range(&self) -> (String, String) {
        (to_rfc3339(self.0.start), to_rfc3339(self.0.end))
    }

    fn from_rfc3339_range(start: &str, end: &str) -> Result<Self, chrono::ParseError> {
        let start_time = from_rfc3339(start)?;
        let end_time = from_rfc3339(end)?;
        Ok(TimeRange(start_time..end_time))
    }
}

// Utils
fn to_rfc3339(t: SystemTime) -> String {
    let datetime: DateTime<Utc> = t.into();
    datetime.to_rfc3339()
}

fn from_rfc3339(s: &str) -> Result<SystemTime, chrono::ParseError> {
    let dt = DateTime::parse_from_rfc3339(s)?;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(dt.timestamp() as u64))
}

// Impl serde Serialize/Deserialize pour TimeRange
impl Serialize for TimeRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (start, end) = self.to_rfc3339_range();
        (start, end).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TimeRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (start, end): (String, String) = Deserialize::deserialize(deserializer)?;
        TimeRange::from_rfc3339_range(&start, &end).map_err(de::Error::custom)
    }
}
