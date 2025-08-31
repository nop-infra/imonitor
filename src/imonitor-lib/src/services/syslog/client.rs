use super::errors::SyslogError;
use crate::device::Device;
use idevice::{IdeviceService, syslog_relay::SyslogRelayClient};
use logger::HasLogger;
use logger::{debug, error, info};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;
use tokio::time::{Duration, sleep, timeout};

const RETRY_CONNECT_WAIT_SECS: u64 = 5;
const SYSLOG_FILE_NAME: &str = "syslog.log";

impl Device {
    pub async fn stream_syslog(
        &self,
        refresh_rate: Duration,
        hb_connected_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), SyslogError> {
        let provider = self.get_provider("syslog");

        let mut _interval = refresh_rate.as_secs();

        let syslog_base_path = PathBuf::from(self.get_syslog_dir());
        let syslog_file_path = syslog_base_path.join(SYSLOG_FILE_NAME);
        let mut f = BufWriter::new(
            File::options()
                .append(true)
                .create(true)
                .open(syslog_file_path)
                .await
                .map_err(SyslogError::OpenFile)?,
        );

        loop {
            // Wait for heartbeat connected state
            if hb_connected_rx.wait_for(|val| *val).await.is_ok()
                && let Ok(connection) = timeout(
                    Duration::from_secs(2),
                    SyslogRelayClient::connect(&*provider),
                )
                .await
            {
                debug!(self, "Connecting syslog");
                // Got response before timeout
                match connection {
                    Ok(mut client) => {
                        info!(self, "Syslog connected");
                        loop {
                            match write_log(&mut client, &mut f, hb_connected_rx).await {
                                Err(e) => match e {
                                    SyslogError::Connect(err) => {
                                        error!(self, "Service needs reconnecting, retrying: {err}");
                                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                                        break;
                                    }
                                    SyslogError::Timeout => {
                                        error!(
                                            self,
                                            "Service needs reconnecting (timeout), retrying"
                                        );
                                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
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
                                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                                        break;
                                    } else {
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(self, "Failed to connect to syslog: {e}");
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                        continue;
                    }
                }
            } else {
                debug!(self, "Syslog connection timeout");
                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
            }
        }
    }
}

async fn write_log<T>(
    client: &mut SyslogRelayClient,
    writer: &mut T,
    hb_connected_rx: &mut watch::Receiver<bool>,
) -> Result<bool, SyslogError>
where
    T: tokio::io::AsyncWrite + std::marker::Unpin,
{
    let mut hb_rx = hb_connected_rx.clone();
    let res = tokio::select!(
        ok = async {
            // Heartbeat lost
            hb_rx.changed().await
                .map_err(SyslogError::HeartbeatWatch)?;
            // Heartbeat retrieved, no need to continue streaming
            hb_rx.wait_for(|val| *val).await
                .map_err(SyslogError::HeartbeatWatch)?;
            Ok::<_,SyslogError>(())
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
        let mut log = log.map_err(SyslogError::Connect)?;
        log.push('\n');
        writer
            .write_all(log.as_bytes())
            .await
            .map_err(SyslogError::WriteToFile)?;
        // No new heartbeat, continue streaming logs
        Ok(false)
    } else {
        // New heartbeat, init new syslog connection
        Ok(true)
    }
}
