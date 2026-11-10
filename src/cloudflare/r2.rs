//! Cloudflare R2 storage via its S3-compatible API.

use aws_sdk_s3::{config::Credentials, primitives::ByteStream, Client};
use std::path::{Path, PathBuf};

type BoxError = Box<dyn std::error::Error>;

pub struct R2 {
    client: Client,
}

impl R2 {
    pub fn from_env() -> Self {
        let account = std::env::var("CF_ACCOUNT_ID").expect("CF_ACCOUNT_ID not set");
        let creds = Credentials::new(
            std::env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID not set"),
            std::env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY not set"),
            None,
            None,
            "r2",
        );
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(aws_sdk_s3::config::Region::new("auto"))
            .endpoint_url(format!("https://{account}.r2.cloudflarestorage.com"))
            .credentials_provider(creds)
            .build();
        R2 { client: Client::from_conf(config) }
    }

    /// Stream an object to a local file.
    pub async fn download(&self, bucket: &str, key: &str, dest: &Path) -> Result<(), BoxError> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let obj = self.client.get_object().bucket(bucket).key(key).send().await?;
        let mut reader = obj.body.into_async_read();
        let mut file = tokio::fs::File::create(dest).await?;
        tokio::io::copy(&mut reader, &mut file).await?;
        Ok(())
    }

    /// Upload every file under `dir` to `bucket` at `prefix/<relative path>`.
    pub async fn upload_dir(&self, bucket: &str, dir: &Path, prefix: &str) -> Result<(), BoxError> {
        for path in list_files(dir)? {
            let rel = path.strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
            let key = format!("{}/{}", prefix.trim_end_matches('/'), rel);
            self.client
                .put_object()
                .bucket(bucket)
                .key(&key)
                .content_type(content_type(&path))
                .body(ByteStream::from_path(&path).await?)
                .send()
                .await?;
        }
        Ok(())
    }
}

fn list_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.extend(list_files(&path)?);
        } else {
            out.push(path);
        }
    }
    Ok(out)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("m3u8") => "application/vnd.apple.mpegurl",
        Some("ts") => "video/mp2t",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("vtt") => "text/vtt",
        _ => "application/octet-stream",
    }
}
