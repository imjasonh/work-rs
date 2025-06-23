use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct FileMapping {
    pub filename: String,
    pub sha256: String,
    pub size: usize,
    pub content_type: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize)]
struct MappingRequest {
    sha256: String,
    size: usize,
    content_type: Option<String>,
}

#[durable_object]
pub struct FileMappingObject {
    state: State,
    _env: Env,
}

impl DurableObject for FileMappingObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, _env: env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let url = req.url()?;
        let path = url.path();

        console_log!("FileMappingObject fetch called for path: {}", path);

        // Extract filename from path
        let filename = path.strip_prefix("/").unwrap_or("");

        match req.method() {
            Method::Get => {
                // Get mapping for a filename
                if filename.is_empty() {
                    // For now, return empty list since storage.list() is not available
                    // In production, you would need to maintain a separate index of all keys
                    let mappings: Vec<FileMapping> = Vec::new();
                    Response::from_json(&mappings)
                } else {
                    // Get specific mapping
                    let storage = self.state.storage();
                    match storage.get::<FileMapping>(filename).await {
                        Ok(mapping) => Response::from_json(&mapping),
                        Err(_) => Response::error("Mapping not found", 404),
                    }
                }
            }
            Method::Put => {
                // Create or update mapping
                if filename.is_empty() {
                    return Response::error("Filename required", 400);
                }

                let body = req.text().await?;
                let request: MappingRequest = serde_json::from_str(&body)
                    .map_err(|e| Error::RustError(format!("Invalid JSON: {}", e)))?;

                let now = js_sys::Date::now() as u64;

                // Check if mapping already exists
                let storage = self.state.storage();
                let existing = storage.get::<FileMapping>(filename).await.ok();

                let mapping = FileMapping {
                    filename: filename.to_string(),
                    sha256: request.sha256,
                    size: request.size,
                    content_type: request.content_type,
                    created_at: existing.as_ref().map(|m| m.created_at).unwrap_or(now),
                    updated_at: now,
                };

                // Check if content has changed
                let changed = existing
                    .as_ref()
                    .map_or(true, |m| m.sha256 != mapping.sha256);

                // Save mapping
                storage.put(filename, &mapping).await?;

                let mut response = Response::from_json(&mapping)?;
                if !changed {
                    response = response.with_status(304); // Not Modified
                }

                Ok(response)
            }
            Method::Delete => {
                // Delete mapping
                if filename.is_empty() {
                    return Response::error("Filename required", 400);
                }

                let storage = self.state.storage();
                storage.delete(filename).await?;

                Response::ok("Mapping deleted")
            }
            _ => Response::error("Method not allowed", 405),
        }
    }
}
