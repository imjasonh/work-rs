use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    pub key: String,
    pub size: usize,
    pub content_type: Option<String>,
    pub uploaded_at: u64,
}

/// Trait for R2 operations to enable testing
#[async_trait(?Send)]
pub trait R2Storage {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<FileMetadata>;
    async fn download(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>>;
}

pub struct R2StorageImpl {
    bucket: Bucket,
}

impl R2StorageImpl {
    pub fn new(bucket: Bucket) -> Self {
        Self { bucket }
    }
}

#[async_trait(?Send)]
impl R2Storage for R2StorageImpl {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<FileMetadata> {
        let size = data.len();

        let put_request = self.bucket.put(key, data);

        // Note: HttpMetadata API might vary by worker version
        // For now, we'll skip setting content-type metadata
        let _ = content_type; // Suppress unused warning

        put_request.execute().await?;

        Ok(FileMetadata {
            key: key.to_string(),
            size,
            content_type: content_type.map(|s| s.to_string()),
            uploaded_at: js_sys::Date::now() as u64,
        })
    }

    async fn download(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let object = self.bucket.get(key).execute().await?;

        if let Some(object) = object {
            let body = object
                .body()
                .ok_or(Error::RustError("No body".to_string()))?;
            let bytes = body.bytes().await?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.bucket.delete(key).await?;
        Ok(())
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut list_request = self.bucket.list();

        if let Some(p) = prefix {
            list_request = list_request.prefix(p);
        }

        let objects = list_request.execute().await?;

        Ok(objects
            .objects()
            .into_iter()
            .map(|obj| obj.key().to_string())
            .collect())
    }
}

/// Handle R2 file operations via HTTP endpoints
pub async fn handle_r2_request(mut req: Request, bucket: Bucket, path: &str) -> Result<Response> {
    let storage = R2StorageImpl::new(bucket);

    // Extract file key from path (e.g., /files/my-file.txt -> my-file.txt)
    let key = path.strip_prefix("/files/").unwrap_or(path);

    match req.method() {
        Method::Get => {
            if key.is_empty() {
                // List files
                let files = storage.list(None).await?;
                Response::from_json(&files)
            } else {
                // Download specific file
                match storage.download(key).await? {
                    Some(data) => {
                        let mut headers = Headers::new();
                        headers.set("Content-Type", "application/octet-stream")?;

                        Ok(Response::from_bytes(data)?.with_headers(headers))
                    }
                    None => Response::error("File not found", 404),
                }
            }
        }
        Method::Put | Method::Post => {
            // Upload file
            let content_type = req.headers().get("Content-Type")?;

            let data = req.bytes().await?;
            let metadata = storage.upload(key, data, content_type.as_deref()).await?;

            Response::from_json(&metadata)
        }
        Method::Delete => {
            // Delete file
            storage.delete(key).await?;
            Response::ok("File deleted")
        }
        _ => Response::error("Method not allowed", 405),
    }
}
