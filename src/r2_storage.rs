use crate::file_mapping_object::FileMapping;
use crate::sha256::compute_sha256;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use worker::*;

#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    pub key: String,
    pub size: usize,
    pub content_type: Option<String>,
    pub uploaded_at: u64,
    pub sha256: Option<String>,
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
    async fn download(&self, key: &str) -> Result<Option<(Vec<u8>, String, Option<String>)>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>>;
}

pub struct R2StorageImpl {
    bucket: Bucket,
    env: Env,
}

impl R2StorageImpl {
    pub fn new(bucket: Bucket, env: Env) -> Self {
        Self { bucket, env }
    }

    async fn get_file_mapping_stub(&self) -> Result<worker::durable::Stub> {
        let namespace = self.env.durable_object("FILE_MAPPING_OBJECT")?;
        let id = namespace.id_from_name("global")?; // Single global mapping instance
        id.get_stub()
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

        // Compute SHA256 of the content
        console_log!("Computing SHA256 for key: {}", key);
        let sha256 = compute_sha256(&data).await?;
        let blob_key = format!("blobs/{}", &sha256);

        console_log!("SHA256 for {}: {}", key, &sha256);

        // Check if blob already exists using conditional put
        let existing_object = self.bucket.get(&blob_key).execute().await?;

        if existing_object.is_none() {
            // Blob doesn't exist, write it
            console_log!("Writing new blob: {}", blob_key);
            let mut put_request = self.bucket.put(&blob_key, data);

            // Add content-type to blob metadata if provided
            if let Some(ct) = content_type {
                let metadata = HttpMetadata {
                    content_type: Some(ct.to_string()),
                    ..Default::default()
                };
                put_request = put_request.http_metadata(metadata);
            }

            put_request.execute().await?;
        } else {
            console_log!("Blob already exists: {}", blob_key);
        }

        // Update the filename->SHA256 mapping in the Durable Object
        let stub = self.get_file_mapping_stub().await?;

        let mapping_request = serde_json::json!({
            "sha256": &sha256,
            "size": size,
            "content_type": content_type
        });

        let request = Request::new_with_init(
            &format!("https://fake-host/{}", key),
            RequestInit::new()
                .with_method(Method::Put)
                .with_body(Some(JsValue::from_str(&mapping_request.to_string())))
                .with_headers({
                    let headers = Headers::new();
                    headers.set("content-type", "application/json")?;
                    headers
                }),
        )?;

        let response = stub.fetch_with_request(request).await?;

        if response.status_code() >= 400 {
            return Err(Error::RustError(format!(
                "Failed to update file mapping: {}",
                response.status_code()
            )));
        }

        Ok(FileMetadata {
            key: key.to_string(),
            size,
            content_type: content_type.map(|s| s.to_string()),
            uploaded_at: js_sys::Date::now() as u64,
            sha256: Some(sha256),
        })
    }

    async fn download(&self, key: &str) -> Result<Option<(Vec<u8>, String, Option<String>)>> {
        // First, get the SHA256 from the mapping
        let stub = self.get_file_mapping_stub().await?;

        let request = Request::new_with_init(
            &format!("https://fake-host/{}", key),
            RequestInit::new().with_method(Method::Get),
        )?;

        let mut response = stub.fetch_with_request(request).await?;

        if response.status_code() == 404 {
            // No mapping found, file doesn't exist
            return Ok(None);
        }

        if response.status_code() >= 400 {
            return Err(Error::RustError(format!(
                "Failed to get file mapping: {}",
                response.status_code()
            )));
        }

        let mapping: FileMapping = response.json().await?;
        let sha256 = mapping.sha256.clone();
        let blob_key = format!("blobs/{}", sha256);

        // Now fetch the actual blob
        let object = self.bucket.get(&blob_key).execute().await?;

        if let Some(object) = object {
            let body = object
                .body()
                .ok_or(Error::RustError("No body".to_string()))?;
            let bytes = body.bytes().await?;

            // Verify the blob content matches the expected SHA256
            let actual_sha256 = compute_sha256(&bytes).await?;
            if actual_sha256 != sha256 {
                return Err(Error::RustError(format!(
                    "Blob integrity check failed for {}. Expected: {}, Actual: {}",
                    key, sha256, actual_sha256
                )));
            }

            Ok(Some((bytes, sha256, mapping.content_type)))
        } else {
            // Blob is missing but mapping exists - likely deleted by R2 lifecycle
            // Clean up the orphaned mapping
            console_log!(
                "Blob {} not found for file {}. Cleaning up orphaned mapping.",
                sha256,
                key
            );

            let delete_request = Request::new_with_init(
                &format!("https://fake-host/{}", key),
                RequestInit::new().with_method(Method::Delete),
            )?;

            let _ = stub.fetch_with_request(delete_request).await;

            // Return None to indicate file not found
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // Delete the mapping from the Durable Object
        let stub = self.get_file_mapping_stub().await?;

        let request = Request::new_with_init(
            &format!("https://fake-host/{}", key),
            RequestInit::new().with_method(Method::Delete),
        )?;

        stub.fetch_with_request(request).await?;

        // Note: We don't delete the blob itself as it might be referenced by other files
        // A garbage collection process could be implemented separately

        Ok(())
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        // Get all mappings from the Durable Object
        let stub = self.get_file_mapping_stub().await?;

        let request = Request::new_with_init(
            "https://fake-host/",
            RequestInit::new().with_method(Method::Get),
        )?;

        let mut response = stub.fetch_with_request(request).await?;

        if response.status_code() >= 400 {
            return Err(Error::RustError(format!(
                "Failed to list file mappings: {}",
                response.status_code()
            )));
        }

        let mappings: Vec<FileMapping> = response.json().await?;

        // Filter by prefix if provided
        let files: Vec<String> = mappings
            .into_iter()
            .filter(|m| {
                if let Some(p) = prefix {
                    m.filename.starts_with(p)
                } else {
                    true
                }
            })
            .map(|m| m.filename)
            .collect();

        Ok(files)
    }
}

/// Handle R2 file operations via HTTP endpoints
pub async fn handle_r2_request(mut req: Request, env: Env, path: &str) -> Result<Response> {
    let bucket = env.bucket("FILES_BUCKET")?;
    let storage = R2StorageImpl::new(bucket, env);

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
                    Some((data, sha256, content_type)) => {
                        let headers = Headers::new();
                        headers.set(
                            "Content-Type",
                            content_type
                                .as_deref()
                                .unwrap_or("application/octet-stream"),
                        )?;
                        // Add Content-Digest header with SHA-256
                        headers.set(
                            "Content-Digest",
                            &format!("sha-256=:{}:", base64_encode(&hex_to_bytes(&sha256)?)),
                        )?;

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
        Method::Head => {
            // HEAD request - return headers without body
            if key.is_empty() {
                Response::error("Method not allowed for listing", 405)
            } else {
                match storage.download(key).await? {
                    Some((data, sha256, content_type)) => {
                        let headers = Headers::new();
                        headers.set(
                            "Content-Type",
                            content_type
                                .as_deref()
                                .unwrap_or("application/octet-stream"),
                        )?;
                        headers.set("Content-Length", &data.len().to_string())?;
                        // Add Content-Digest header with SHA-256
                        headers.set(
                            "Content-Digest",
                            &format!("sha-256=:{}:", base64_encode(&hex_to_bytes(&sha256)?)),
                        )?;

                        // Return empty response with headers only
                        Ok(Response::empty()?.with_headers(headers))
                    }
                    None => Response::error("File not found", 404),
                }
            }
        }
        _ => Response::error("Method not allowed", 405),
    }
}

/// Convert hex string to bytes
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| Error::RustError("Invalid hex string".to_string()))
        })
        .collect()
}

/// Base64 encode bytes
fn base64_encode(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(bytes)
}
