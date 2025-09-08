use aws_config::retry::RetryConfig;
use aws_config::{BehaviorVersion, defaults};
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::ByteStream;
use chrono::Utc;
use serde::Deserialize;
use std::{env, error::Error, fs, io::SeekFrom};
use tokio::io::AsyncReadExt;
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    time::{Duration, sleep},
};
use tracing::{error, info, warn};

#[derive(Deserialize)]
struct Config {
    log_file_path: String,
    chunk_size_mb: usize,
    check_interval_seconds: u64,
    s3: S3Config,
}

#[derive(Deserialize)]
struct S3Config {
    bucket: String,
    prefix: String,
    endpoint: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    // Load config file
    let config: Config = load_config("config.toml")?;

    // Load secrets from env
    let access_key = env::var("S3_ACCESS_KEY")?;
    let secret_key = env::var("S3_SECRET_KEY")?;

    let client = build_s3_client(&config, &access_key, &secret_key).await?;

    loop {
        if let Err(e) = process_log_file(&client, &config).await {
            error!("Error processing log file: {:?}", e);
        }
        sleep(Duration::from_secs(config.check_interval_seconds)).await;
    }
}

fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

async fn build_s3_client(
    config: &Config,
    access_key: &str,
    secret_key: &str,
) -> Result<Client, Box<dyn Error>> {
    let retry_config = RetryConfig::standard().with_max_attempts(5);

    let aws_config = defaults(BehaviorVersion::latest())
        .retry_config(retry_config)
        .endpoint_url(&config.s3.endpoint)
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            access_key, secret_key, None, None, "static",
        ))
        .load()
        .await;

    Ok(Client::new(&aws_config))
}

async fn process_all_logs(client: &Client, config: &Config) -> Result<(), Box<dyn Error>> {
    // Find all logs in dirs
    // Process logs
}

async fn process_log_file(client: &Client, config: &Config) -> Result<(), Box<dyn Error>> {
    let metadata = tokio::fs::metadata(&config.log_file_path).await?;
    let file_size_mb = metadata.len() / (1024 * 1024);

    if file_size_mb <= config.chunk_size_mb.try_into()? {
        return Ok(());
    }

    let chunk_size_bytes = config.chunk_size_mb * 1024 * 1024;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&config.log_file_path)
        .await?;

    let mut reader = BufReader::new(&mut file);
    reader.seek(SeekFrom::Start(0)).await?;

    let mut buffer = Vec::with_capacity(chunk_size_bytes);
    let mut bytes_read = 0;

    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        let len = line.len() + 1;
        if bytes_read + len > chunk_size_bytes {
            break;
        }
        bytes_read += len;
        buffer.extend_from_slice(line.as_bytes());
        buffer.push(b'\n');
    }

    if buffer.is_empty() {
        return Ok(());
    }

    let s3_key = format!(
        "{}{}.log",
        config.s3.prefix,
        Utc::now().format("%Y%m%d-%H%M%S")
    );

    upload_to_s3_with_retries(client, &config.s3.bucket, &s3_key, buffer).await?;

    truncate_file_preserving_tail(&mut file, bytes_read).await?;

    Ok(())
}

async fn truncate_file_preserving_tail(
    file: &mut tokio::fs::File,
    offset: usize,
) -> Result<(), Box<dyn Error>> {
    let mut remaining = Vec::new();
    file.seek(SeekFrom::Start(offset as u64)).await?;
    file.read_to_end(&mut remaining).await?;

    file.set_len(0).await?;
    file.seek(SeekFrom::Start(0)).await?;
    file.write_all(&remaining).await?;
    file.flush().await?;

    info!("Truncated {} bytes from beginning of log", offset);
    Ok(())
}

async fn upload_to_s3_with_retries(
    client: &Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    for attempt in 0..5 {
        match upload_to_s3(client, bucket, key, data.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                warn!("S3 upload failed (attempt {}/5): {:?}", attempt + 1, e);
                let delay = 500 * (2_u64.pow(attempt as u32));
                sleep(Duration::from_millis(delay)).await;
            }
        }
    }
    Err("All upload attempts failed".into())
}

async fn upload_to_s3(
    client: &Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    let stream = ByteStream::from(data);
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(stream)
        .send()
        .await?;

    info!("Uploaded to S3: {}", key);
    Ok(())
}
