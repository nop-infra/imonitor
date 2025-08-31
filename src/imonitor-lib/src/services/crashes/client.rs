use super::errors::CrashError;
use crate::device::Device;
use idevice::{
    IdeviceError, IdeviceService, afc::errors::AfcError,
    crashreportcopymobile::CrashReportCopyMobileClient,
};
use logger::{HasLogger, debug, error, info};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs::{File, create_dir_all, read_to_string, try_exists};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::watch;
use tokio::time::{Duration, sleep, timeout};

const RETRY_CONNECT_WAIT_SECS: u64 = 15;
const POLL_WAIT_SECS: u64 = 15;
const KNOWN_CRASHES_FILE_NAME: &str = "known_crashes.json";
const KNOWN_CRASH_DIRS_FILE_NAME: &str = "known_dirs.json";

impl Device {
    pub async fn get_crashes(
        &self,
        refresh_rate: Duration,
        hb_connected_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), CrashError> {
        let mut _interval = refresh_rate.as_secs();

        // Get already known crashes
        self.get_known_crashes_from_fs().await?;

        loop {
            let provider = self.get_provider("crashes");

            // Wait for heartbeat connected state
            if hb_connected_rx.wait_for(|val| *val).await.is_ok()
                && let Ok(connection) = timeout(
                    Duration::from_secs(2),
                    CrashReportCopyMobileClient::connect(&*provider),
                )
                .await
            {
                debug!(self, "Connecting to crash report service");
                // Got response before timeout
                match connection {
                    Ok(mut client) => {
                        info!(self, "Crash service connected");
                        loop {
                            if let Err(e) = self.write_crashes(&mut client).await {
                                match e {
                                    CrashError::Connect(err) => {
                                        error!(
                                            self,
                                            "Service needs reconnecting, retrying : {err}"
                                        );
                                        break;
                                    }
                                    CrashError::Timeout => {
                                        error!(
                                            self,
                                            "Service needs reconnecting (timeout), retrying"
                                        );
                                        break;
                                    }
                                    err => {
                                        error!(self, "Failed to write crashes: {err}");
                                        break;
                                    }
                                }
                            } else {
                                sleep(Duration::from_secs(POLL_WAIT_SECS)).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(self, "Failed to connect to crashes service : {e}");
                        sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                        continue;
                    }
                }
            } else {
                debug!(self, "Service connection timeout");
                sleep(Duration::from_secs(RETRY_CONNECT_WAIT_SECS)).await;
                continue;
            }
        }
    }

    pub async fn get_known_crashes_from_fs(&self) -> Result<(), CrashError> {
        let crashes_file_path = self.get_known_crashes_file_path();
        let crash_dirs_file_path = self.get_known_crash_dirs_file_path();

        if try_exists(&crashes_file_path)
            .await
            .map_err(|e| CrashError::FileExists(e, crashes_file_path.clone()))?
        {
            let content = read_to_string(crashes_file_path.clone())
                .await
                .map_err(|e| CrashError::ReadFile(e, crashes_file_path.clone()))?;

            let known_crashes: HashSet<String> =
                serde_json::from_str(&content).map_err(CrashError::DeserializeKnownCrashes)?;

            {
                let mut crash_files = self
                    .crashes
                    .crash_files
                    .write()
                    .map_err(|_| CrashError::WriteLock)?;

                *crash_files = known_crashes.clone();
            }
        }

        if try_exists(&crash_dirs_file_path)
            .await
            .map_err(|e| CrashError::FileExists(e, crash_dirs_file_path.clone()))?
        {
            let content = read_to_string(crash_dirs_file_path.clone())
                .await
                .map_err(|e| CrashError::ReadFile(e, crash_dirs_file_path.clone()))?;

            let known_crash_dirs: HashSet<String> =
                serde_json::from_str(&content).map_err(CrashError::DeserializeKnownCrashes)?;

            {
                let mut crash_dirs = self
                    .crashes
                    .crash_dirs
                    .write()
                    .map_err(|_| CrashError::WriteLock)?;

                *crash_dirs = known_crash_dirs.clone();
            }
        }

        Ok(())
    }

    pub async fn write_crashes(
        &self,
        client: &mut CrashReportCopyMobileClient,
    ) -> Result<(), CrashError> {
        // List all files
        // TODO : add timeout
        let mut files = HashSet::<String>::from_iter(
            client
                .ls(None)
                .await
                .map_err(|e| CrashError::ListFiles(e, "".to_string()))?,
        );

        debug!(self, "Files found: {}", files.len());
        {
            // Get crash dirs from device struct
            let crash_dirs;
            {
                crash_dirs = self
                    .crashes
                    .crash_dirs
                    .read()
                    .map_err(|_| CrashError::ReadLock)?
                    .clone();
            }

            // List files in all dirs
            for dir in crash_dirs {
                files.extend(HashSet::<String>::from_iter(
                    //TODO : add timeout
                    client
                        .ls(Some(&dir))
                        .await
                        .map_err(|e| {
                            // Remove dir from known dirs if listing fails
                            // Ensures deleted dirs do not fail in loop
                            let mut crash_dirs = self
                                .crashes
                                .crash_dirs
                                .write()
                                .map_err(|_| CrashError::WriteLock);

                            match crash_dirs {
                                Ok(ref mut set) => {
                                    set.remove(&dir);
                                    CrashError::ListFiles(e, dir.to_string())
                                }
                                Err(_) => CrashError::ListFiles(e, dir.to_string()),
                            }
                        })?
                        .iter()
                        .filter_map(|file| {
                            if file == "."
                                || file == ".."
                                || file.starts_with("IN_PROGRESS_sysdiagnose_")
                            {
                                None
                            } else {
                                let dir_path = Path::new(&dir);
                                Some(dir_path.join(file).to_string_lossy().to_string())
                            }
                        }),
                ))
            }
        }

        let files_to_get;
        {
            let crash_files = self
                .crashes
                .crash_files
                .read()
                .map_err(|_| CrashError::ReadLock)?;

            let crash_dirs = self
                .crashes
                .crash_dirs
                .read()
                .map_err(|_| CrashError::ReadLock)?;

            files_to_get = files
                .difference(&crash_files)
                .cloned()
                .collect::<HashSet<String>>()
                .difference(&crash_dirs)
                .cloned()
                .collect::<HashSet<String>>();

            debug!(self, "Remaining files to get: {}", files_to_get.len());
        }

        let mut files_give_up = HashSet::<String>::new();

        // Files to download
        for file in files_to_get {
            info!(self, "File : {file:?}");
            let root_dir = PathBuf::from(self.get_crash_files_dir());
            let dst_file_path = root_dir.join(&file);

            // Try to pull file from device
            // TODO : add timeout
            let content = match client.pull(file.clone()).await {
                Ok(content) => content,
                Err(e) => {
                    // Check if path is a dir
                    match client.afc_client.get_file_info(file.clone()).await {
                        Ok(file_info) => {
                            if file_info.st_ifmt == "S_IFDIR" {
                                debug!(self, "Directory found : {file}");
                                // Update known dirs
                                {
                                    let mut crash_dirs_mut = self
                                        .crashes
                                        .crash_dirs
                                        .write()
                                        .map_err(|_| CrashError::WriteLock)?;

                                    crash_dirs_mut.insert(file.clone());
                                }
                            } else {
                                match e {
                                    IdeviceError::Afc(AfcError::ObjectNotFound)
                                    | IdeviceError::Afc(AfcError::PermDenied) => {
                                        files_give_up.insert(file.clone());
                                    }
                                    _ => {}
                                }
                                error!(self, "Failed to pull file : {e}");
                            }
                        }
                        Err(e) => error!(self, "Failed to get file info for {file}: {e}"),
                    }
                    continue;
                }
            };

            // Write file to filesystem
            match write_file(&content, &dst_file_path).await {
                Ok(_) => {
                    let mut crash_files = self
                        .crashes
                        .crash_files
                        .write()
                        .map_err(|_| CrashError::WriteLock)?;

                    crash_files.insert(file.clone());
                }
                Err(e) => {
                    error!(self, "Failed to write file {file}: {e}");
                    continue;
                }
            }
            self.update_known_crashes(
                &files
                    .difference(&files_give_up)
                    .cloned()
                    .collect::<HashSet<String>>(),
            )
            .await?;
        }

        Ok(())
    }

    pub fn get_known_crashes_file_path(&self) -> String {
        let crashes_dir = PathBuf::from(self.get_crashes_dir());
        let known_crashes_file_path = crashes_dir.join(KNOWN_CRASHES_FILE_NAME);
        known_crashes_file_path.to_string_lossy().to_string()
    }

    pub fn get_known_crash_dirs_file_path(&self) -> String {
        let crashes_dir = PathBuf::from(self.get_crashes_dir());
        let known_crash_dirs_file_path = crashes_dir.join(KNOWN_CRASH_DIRS_FILE_NAME);
        known_crash_dirs_file_path.to_string_lossy().to_string()
    }

    pub async fn update_known_crashes(&self, _files: &HashSet<String>) -> Result<(), CrashError> {
        let crash_dirs: HashSet<String>;
        {
            let crash_dirs_orig = self
                .crashes
                .crash_dirs
                .read()
                .map_err(|_| CrashError::ReadLock)?;

            crash_dirs = crash_dirs_orig.clone();
        }

        let crash_files;
        {
            crash_files = self
                .crashes
                .crash_files
                .read()
                .map_err(|_| CrashError::ReadLock)?
                .clone();
        }
        /*
         * TODO : fix the following code
         * We cannot clean the files as we are not sure we visited all the files we already knew
         * The problem is as it's asynchronous code, we may update known crashes before visiting
         * all dirs
         * If that's the case, we lose some crashes we've known before, so we download them again
        let crash_files_cleaned;
        {
            let mut crash_files = self
                .crashes
                .crash_files
                .write()
                .map_err(|_| CrashError::WriteLock)?;

            crash_files_cleaned = crash_files
                .intersection(files)
                .cloned()
                .collect::<HashSet<String>>();

            *crash_files = crash_files_cleaned.clone();
        }
        */

        let known_crashes_file_path = self.get_known_crashes_file_path();
        let known_crash_dirs_file_path = self.get_known_crash_dirs_file_path();

        //let crash_files_content = serde_json::to_string_pretty(&crash_files_cleaned)
        let crash_files_info = (
            serde_json::to_string_pretty(&crash_files)
                .map_err(CrashError::SerializeKnownCrashes)?,
            known_crashes_file_path,
        );

        let crash_dirs_info = (
            serde_json::to_string_pretty(&crash_dirs).map_err(CrashError::SerializeKnownCrashes)?,
            known_crash_dirs_file_path,
        );

        for (content, output_file_path) in [crash_files_info, crash_dirs_info] {
            let file_h = File::create(output_file_path.clone())
                .await
                .map_err(|e| CrashError::CreateFile(e, output_file_path.clone()))?;

            let mut writer = BufWriter::new(file_h);

            writer
                .write_all(content.as_bytes())
                .await
                .map_err(|e| CrashError::WriteToFile(e, output_file_path.clone()))?;

            writer
                .flush()
                .await
                .map_err(|e| CrashError::WriteToFile(e, output_file_path.clone()))?;
        }
        Ok(())
    }
}

async fn write_file(content: &[u8], dst_file_path: &PathBuf) -> Result<(), CrashError> {
    let dst_file_path_string = dst_file_path.to_string_lossy().to_string();

    if let Some(dir) = dst_file_path.parent() {
        let dir_string = dir.to_string_lossy().to_string();
        create_dir_all(&dir_string)
            .await
            .map_err(|e| CrashError::CreateDir(e, dir_string.clone()))?;
    }

    let dst_file = File::create(dst_file_path)
        .await
        .map_err(|e| CrashError::CreateFile(e, dst_file_path_string.clone()))?;

    let mut writer = BufWriter::new(dst_file);
    writer
        .write_all(content)
        .await
        .map_err(|e| CrashError::WriteToFile(e, dst_file_path_string.clone()))?;

    writer
        .flush()
        .await
        .map_err(|e| CrashError::WriteToFile(e, dst_file_path_string.clone()))
}
