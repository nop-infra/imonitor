use super::archive::extract_time_coverage_from_tar;
use super::errors::OsTraceError;
use crate::device::Device;
use crate::device::activity_coverage::ActivityCoverage;
use chrono::{DateTime, Utc};
use idevice::{
    IdeviceService,
    services::os_trace_relay::OsTraceLog,
    services::os_trace_relay::{OsTraceRelayClient, OsTraceRelayReceiver},
};
use logger::HasLogger;
use logger::{debug, error, info};
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;
use tokio::time::{Duration, sleep, timeout};

const RETRY_CONNECT_WAIT_SECS: u64 = 5;
const OS_TRACE_LOG_FILE_NAME: &str = "os_trace_log.json";
const ARCHIVE_EXTENSION: &str = "tar";

impl Device {
    pub async fn stream_os_trace_logs(
        &self,
        refresh_rate: Duration,
        hb_connected_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), OsTraceError> {
        let provider = self.get_provider("os_trace_log");

        let mut _interval = refresh_rate.as_secs();

        let log_base_path = PathBuf::from(self.get_os_trace_log_dir());
        let log_file_path = log_base_path.join(OS_TRACE_LOG_FILE_NAME);
        let mut f = BufWriter::new(
            File::options()
                .append(true)
                .create(true)
                .open(log_file_path)
                .await
                .map_err(OsTraceError::OpenFile)?,
        );

        loop {
            // Wait for heartbeat connected state
            if hb_connected_rx.wait_for(|val| *val).await.is_ok()
                && let Ok(connection) = timeout(
                    Duration::from_secs(2),
                    OsTraceRelayClient::connect(&*provider),
                )
                .await
            {
                debug!(self, "Connecting os trace log");
                // Got response before timeout
                match connection {
                    Ok(os_trace_client) => {
                        info!(self, "Os trace (log) connected");
                        match os_trace_client.start_trace(None).await {
                            Ok(mut client) => {
                                let interval_start = std::time::SystemTime::now();
                                let mut interval_end;
                                loop {
                                    interval_end = std::time::SystemTime::now();
                                    match write_log(&mut client, &mut f, hb_connected_rx).await {
                                        Err(e) => match e {
                                            OsTraceError::Connect(err) => {
                                                error!(
                                                    self,
                                                    "Service needs reconnecting, retrying: {err}"
                                                );
                                                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS))
                                                    .await;
                                                break;
                                            }
                                            OsTraceError::Timeout => {
                                                error!(
                                                    self,
                                                    "Service needs reconnecting (timeout), retrying"
                                                );
                                                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS))
                                                    .await;
                                                break;
                                            }
                                            err => {
                                                error!(self, "Failed to write logs: {err}");
                                                return Err(err);
                                            }
                                        },
                                        Ok(new_heartbeat) => {
                                            if new_heartbeat {
                                                info!(self, "New heartbeat, reconnecting");
                                                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS))
                                                    .await;
                                                break;
                                            } else {
                                                continue;
                                            }
                                        }
                                    }
                                }
                                // Update activity coverage to calculate gaps
                                let coverage: ActivityCoverage;
                                {
                                    let mut activity_coverage = self
                                        .activity_coverage
                                        .write()
                                        .map_err(|_| OsTraceError::WriteLock)?;
                                    activity_coverage.add_range(interval_start..interval_end);
                                    coverage = activity_coverage.clone();
                                    info!(self, "{activity_coverage:?}");
                                }
                                coverage
                                    .write_to_fs(&self.get_activity_coverage_file_path())
                                    .await?;
                            }
                            Err(e) => {
                                error!(self, "Failed to init log tracing: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        error!(self, "Failed to connect to os trace: {e}");
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                        continue;
                    }
                }
            } else {
                debug!(self, "Os trace connection timeout");
                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
            }
        }
    }

    pub async fn create_os_trace_archive(
        &self,
        refresh_rate: Duration,
        hb_connected_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), OsTraceError> {
        let provider = self.get_provider("os_trace_archive");

        let mut _interval = refresh_rate.as_secs();

        let archive_base_path = PathBuf::from(self.get_os_trace_archive_dir());

        loop {
            let gaps;
            {
                let activity_coverage = self
                    .activity_coverage
                    .read()
                    .map_err(|_| OsTraceError::ReadLock)?;
                gaps = activity_coverage.missing_ranges();
            }
            //
            // Wait for heartbeat connected state
            if !gaps.is_empty()
                && hb_connected_rx.wait_for(|val| *val).await.is_ok()
                && let Ok(connection) = timeout(
                    Duration::from_secs(2),
                    OsTraceRelayClient::connect(&*provider),
                )
                .await
            {
                // Got response before timeout
                match connection {
                    Ok(mut os_trace_client) => {
                        info!(self, "Os trace (archive) connected");

                        info!(self, "Gaps: {gaps:?}");

                        // TODO: calculate gaps only when we know there is a new one (get the
                        // information from a channel, as with heartbeat)
                        for gap in gaps {
                            // Create archive
                            let archive_file_path =
                                archive_base_path.join(self.get_archive_name(&gap.start.into()));
                            let mut f = BufWriter::new(
                                File::options()
                                    .create(true)
                                    .truncate(true)
                                    .write(true)
                                    .open(&archive_file_path)
                                    .await
                                    .map_err(OsTraceError::OpenFile)?,
                            );

                            let archive_start = gap
                                .start
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .map_err(OsTraceError::OppositeTime)?
                                .as_secs();

                            info!(self, "Creating archive beginning at {archive_start}");
                            // Check if archive was finished
                            /*
                            os_trace_client
                                .create_archive(&mut f, Some(5u64), 1, Some(archive_start))
                                .await
                                .map_err(OsTraceError::CreateArchive)?;
                            */
                            if let Err(e) = os_trace_client
                                .create_archive(&mut f, Some(5u64), Some(1), Some(archive_start))
                                .await {
                                info!(self, "Failed to create archive: {e}");
                                sleep(Duration::from_secs(60)).await;
                                continue;
                            } else {

                            let coverage: ActivityCoverage;
                            {
                                let mut activity_coverage = self
                                    .activity_coverage
                                    .write()
                                    .map_err(|_| OsTraceError::WriteLock)?;

                                /*
                                let tar_coverage =
                                    extract_time_coverage_from_tar(&archive_file_path)?;
                                activity_coverage.add_range(gap.start..tar_coverage.end);
                                */
                                activity_coverage.add_range(gap);
                                coverage = activity_coverage.clone();
                            }
                            coverage
                                .write_to_fs(&self.get_activity_coverage_file_path())
                                .await?;
                            info!(self, "Archive created");

                            }
                        }
                        // TODO: get that sleep time from config
                        sleep(Duration::from_secs(60)).await;
                    }
                    Err(e) => {
                        error!(self, "Failed to connect to os trace: {e}");
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                    }
                }
            } else {
                debug!(self, "Os trace connection timeout");
                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
            }
        }
    }

    pub fn get_archive_name(&self, date: &DateTime<Utc>) -> String {
        //let now_utc: DateTime<Utc> = Utc::now();
        let udid = self.info.udid.clone();
        format!(
            "{}_{}.{ARCHIVE_EXTENSION}",
            &udid,
            date.timestamp() //date.to_rfc3339_opts(SecondsFormat::Secs, true)
        )
    }
}

async fn write_log<T>(
    client: &mut OsTraceRelayReceiver,
    writer: &mut T,
    hb_connected_rx: &mut watch::Receiver<bool>,
) -> Result<bool, OsTraceError>
where
    T: tokio::io::AsyncWrite + std::marker::Unpin,
{
    let mut hb_rx = hb_connected_rx.clone();
    let res = tokio::select!(
        ok = async {
            // Heartbeat lost
            hb_rx.changed().await
                .map_err(OsTraceError::HeartbeatWatch)?;
            // Heartbeat retrieved, no need to continue streaming
            hb_rx.wait_for(|val| *val).await
                .map_err(OsTraceError::HeartbeatWatch)?;
            Ok::<_,OsTraceError>(())
        } => {
            ok?;
            None
        },
        log = client.next() => {
           // Log received
           Some(log)
        }
    );

    if let Some(log) = res {
        let mut log_json =
            serde_json::to_string::<OsTraceLog>(&(log.map_err(OsTraceError::Connect)?))
                .map_err(OsTraceError::SerializeLog)?;
        log_json.push('\n');
        writer
            .write_all(log_json.as_bytes())
            .await
            .map_err(OsTraceError::WriteToFile)?;
        // No new heartbeat, continue streaming logs
        Ok(false)
    } else {
        // New heartbeat, init new os trace connection
        Ok(true)
    }
}
