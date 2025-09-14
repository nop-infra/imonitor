use super::errors::HeartbeatError;
use crate::config::Config;
use crate::device::Device;
use chrono::Utc;
use idevice::{IdeviceService, heartbeat::HeartbeatClient};
use logger::{HasLogger, error, info};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;
use tokio::time::{Duration, sleep};

const RETRY_CONNECT_WAIT_SECS: u64 = 30;
const HB_LAST_ESTABLISHED_FILE_NAME: &str = "heartbeat_last_established.json";
const HEARTBEAT_TIMEOUT_SEC: u64 = 7u64;
const HEARTBEAT_NO_RESPONSE_CONSIDER_ALIVE_SEC: u64 = 420u64;

impl Device {
    pub async fn maintain_heartbeat(
        &self,
        config: Arc<RwLock<Config>>,
        connected_sender: &watch::Sender<bool>,
    ) -> Result<(), HeartbeatError> {
        let mut interval;
        {
            interval = config
                .read()
                .map_err(|_| HeartbeatError::ConfigReadLock)?
                .settings
                .clone()
                .refresh_rate
                .as_secs();
        }
        let mut reconnect;

        let provider = self.get_provider("heartbeat");
        loop {
            info!(self, "Connecting to heartbeat");
            tokio::select!(
                biased;
                heartbeat_res = HeartbeatClient::connect(&*provider) => {

                let mut heartbeat_client = match heartbeat_res {
                    Ok(client) => {
                        info!(self, "Heartbeat connection established");
                        reconnect = false;
                        // Ignore error if not updated
                        let _ = self.update_hb_last_established().await;
                        connected_sender
                            .send(true)
                            .map_err(HeartbeatError::SendConnectedState)?;
                        client
                    }
                    Err(e) => {
                        error!(self, "Unable to connect to heartbeat: {e}");
                        connected_sender
                            .send(false)
                            .map_err(HeartbeatError::SendConnectedState)?;
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                        continue;
                    }
                };

                while !reconnect {
                    match heartbeat_client.get_marco(interval).await {
                        Ok(new_interval) => {
                            info!(self, "Heartbeat ok. Interval: {new_interval}");
                            // Wait for message interval + 5 (in case of network failure)
                            interval = new_interval + 5;
                        }
                        Err(e) => {
                            info!(self, "Error getting marco: {e}");
                            reconnect = true;
                            connected_sender
                                .send(false)
                                .map_err(HeartbeatError::SendConnectedState)?;
                        }
                    };

                    if !reconnect && let Err(e) = heartbeat_client.send_polo().await {
                        info!(self, "Error sending polo: {e}");
                    }
                }
                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
            },
            res = async {
                        // Timeout for heartbeat connection
                        sleep(Duration::from_secs(HEARTBEAT_TIMEOUT_SEC)).await;
                        info!(self, "Timeout while connecting to heartbeat, trying to use services either way");
                        connected_sender
                            .send(true)
                            .map_err(HeartbeatError::SendConnectedState)?;
                        sleep(Duration::from_secs(HEARTBEAT_NO_RESPONSE_CONSIDER_ALIVE_SEC)).await;
                        Ok::<(), HeartbeatError>(())
                } => {
                    if let Err(e) = res {
                        info!(self, "Failed to send ok state, while heartbeat timeout: {e}");
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                    } else {
                        continue;
                    }
                },
            )
        }
    }

    pub fn get_hb_last_established_file_path(&self) -> String {
        let crashes_dir = PathBuf::from(self.get_heartbeat_dir());
        let file_path = crashes_dir.join(HB_LAST_ESTABLISHED_FILE_NAME);
        file_path.to_string_lossy().to_string()
    }

    pub async fn update_hb_last_established(&self) -> Result<(), HeartbeatError> {
        let now = Utc::now();

        match self.heartbeat.last_established.clone().write() {
            Ok(mut le) => *le = now,
            Err(_) => {
                error!(
                    self,
                    "Failed to update last established heartbeat date, skipping"
                )
            }
        }

        let heartbeat_file_path_string = self.get_hb_last_established_file_path();

        let content = serde_json::to_string_pretty(&now).map_err(HeartbeatError::SerializeDate)?;

        let dst_file = File::create(heartbeat_file_path_string.clone())
            .await
            .map_err(|e| HeartbeatError::CreateFile(e, heartbeat_file_path_string.clone()))?;

        let mut writer = BufWriter::new(dst_file);

        writer
            .write_all(content.as_bytes())
            .await
            .map_err(|e| HeartbeatError::WriteToFile(e, heartbeat_file_path_string.clone()))?;

        writer
            .flush()
            .await
            .map_err(|e| HeartbeatError::WriteToFile(e, heartbeat_file_path_string.clone()))
    }
}
