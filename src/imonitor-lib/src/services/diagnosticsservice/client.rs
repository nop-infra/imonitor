use super::errors::DiagnosticsServiceError;
use crate::device::Device;
use futures_util::StreamExt;
use idevice::{
    ReadWrite,
    IdeviceService, RsdService, core_device::DiagnostisServiceClient,
    core_device_proxy::CoreDeviceProxy, rsd::RsdHandshake,
};
use logger::HasLogger;
use logger::{debug, error, info};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tokio::time::{Duration, sleep, timeout};

const RETRY_CONNECT_WAIT_SECS: u64 = 5;

impl Device {
    pub async fn get_sysdiagnose(
        &self,
        hb_connected_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), DiagnosticsServiceError> {
        let provider = self.get_provider("get_sysdiagnose");

        let sysdiagnose_base_path = PathBuf::from(self.get_sysdiagnose_dir());

        loop {
            // Wait for heartbeat connected state
            if hb_connected_rx.wait_for(|val| *val).await.is_ok()
                && let Ok(connection) =
                    timeout(Duration::from_secs(2), CoreDeviceProxy::connect(&*provider)).await
            {
                debug!(self, "Connecting sysdiagnose trigger");
                // Got response before timeout
                match connection {
                    Ok(proxy_client) => {
                        info!(self, "Diagnostics service connected");
                        let rsd_port = proxy_client.handshake.server_rsd_port;

                        let adapter = proxy_client
                            .create_software_tunnel()
                            .map_err(|_| DiagnosticsServiceError::CreateSoftwareTunnel)?;

                        let mut adapter = adapter.to_async_handle();

                        let stream = adapter
                            .connect(rsd_port)
                            .await
                            .map_err(|_| DiagnosticsServiceError::ConnectAdapterStream)?;

                        match RsdHandshake::new(stream).await {
                            Ok(mut handshake) => {
                                let mut dsc: DiagnostisServiceClient<Box<dyn ReadWrite + 'static>> = DiagnostisServiceClient::connect_rsd(
                                    &mut adapter,
                                    &mut handshake,
                                )
                                .await
                                .expect("no connect");

                                info!(
                                    self,
                                    "Getting sysdiagnose, this takes a while! iOS is slow..."
                                );

                                let mut _res = dsc
                                    .capture_sysdiagnose(false)
                                    .await
                                    .expect("no sysdiagnose");

                                info!(self, "Got sysdiagnose! Saving to file");

                                /*
                                let sysdiagnose_file_path = sysdiagnose_base_path.join(&res.preferred_filename);
                                let mut written = 0usize;

                                let mut out = tokio::fs::File::create(sysdiagnose_file_path)
                                    .await
                                    .expect("no file?");

                                while let Some(chunk) = res.stream.next().await {
                                    let buf = chunk.expect("stream stopped?");
                                    if !buf.is_empty() {
                                        out.write_all(&buf).await.expect("no write all?");
                                        written += buf.len();
                                    }
                                    info!(self, "wrote {written}/{} bytes", res.expected_length);
                                }
                                info!(self, "Done! Saved to {}", res.preferred_filename);
                                */

                                /*
                                                          tokio::fs::write(
                                                              &rxpc_services_file_path,
                                                              format!("{:#?}", handshake.services).as_bytes(),
                                                          )
                                                          .await
                                                          .map_err(DiagnosticsServiceError::WriteToFile)?;
                                */
                                return Ok(());
                            }
                            Err(err) => {
                                error!(self, "Service needs reconnecting, retrying: {err}");
                                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(self, "Failed to connect to core device proxy: {e}");
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                        continue;
                    }
                }
            } else {
                debug!(self, "Core device proxy connection timeout");
                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
            }
        }
    }
}
