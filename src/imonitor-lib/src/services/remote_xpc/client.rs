use super::errors::RemoteXPCError;
use crate::device::Device;
use idevice::{
    IdeviceService, core_device_proxy::CoreDeviceProxy, rsd::RsdHandshake,
    tcp::stream::AdapterStream,
};
use logger::HasLogger;
use logger::{debug, error, info};
use std::path::PathBuf;
use tokio::sync::watch;
use tokio::time::{Duration, sleep, timeout};

const RETRY_CONNECT_WAIT_SECS: u64 = 5;
const RXPC_SERVICES_FILE_NAME: &str = "remote_xpc_services.json";

impl Device {
    pub async fn discover_remote_xpc_services(
        &self,
        hb_connected_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), RemoteXPCError> {
        let provider = self.get_provider("discover_remote_xpc");

        let rxpc_base_path = PathBuf::from(self.get_remote_xpc_dir());
        let rxpc_services_file_path = rxpc_base_path.join(RXPC_SERVICES_FILE_NAME);

        loop {
            // Wait for heartbeat connected state
            if hb_connected_rx.wait_for(|val| *val).await.is_ok()
                && let Ok(connection) =
                    timeout(Duration::from_secs(2), CoreDeviceProxy::connect(&*provider)).await
            {
                debug!(self, "Connecting os trace log");
                // Got response before timeout
                match connection {
                    Ok(proxy_client) => {
                        info!(self, "Core device proxy connected");
                        let rsd_port = proxy_client.handshake.server_rsd_port;

                        let mut adapter = proxy_client
                            .create_software_tunnel()
                            .map_err(|_| RemoteXPCError::CreateSoftwareTunnel)?;

                        let stream = AdapterStream::connect(&mut adapter, rsd_port)
                            .await
                            .map_err(|_| RemoteXPCError::ConnectAdapterStream)?;

                        match RsdHandshake::new(stream).await {
                            Ok(handshake) => {
                                tokio::fs::write(
                                    &rxpc_services_file_path,
                                    format!("{:#?}", handshake.services).as_bytes(),
                                )
                                .await
                                .map_err(RemoteXPCError::WriteToFile)?;
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
