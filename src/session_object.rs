use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: String,
    pub data: serde_json::Value,
    pub created_at: u64,
    pub updated_at: u64,
}

/// A session storage Durable Object for maintaining user sessions
#[durable_object]
pub struct SessionObject {
    state: State,
    _env: Env,
}

#[durable_object]
impl DurableObject for SessionObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, _env: env }
    }

    async fn fetch(&mut self, mut req: Request) -> Result<Response> {
        let mut storage = self.state.storage();

        match req.method() {
            Method::Get => {
                // Get all session data
                let user_id = match storage.get::<String>("user_id").await {
                    Ok(id) => id,
                    Err(_) => return Response::error("Session not found", 404),
                };
                let data = match storage.get::<serde_json::Value>("data").await {
                    Ok(d) => d,
                    Err(_) => return Response::error("Session not found", 404),
                };
                let created_at = match storage.get::<u64>("created_at").await {
                    Ok(t) => t,
                    Err(_) => return Response::error("Session not found", 404),
                };
                let updated_at = match storage.get::<u64>("updated_at").await {
                    Ok(t) => t,
                    Err(_) => return Response::error("Session not found", 404),
                };

                let session = SessionData {
                    user_id,
                    data,
                    created_at,
                    updated_at,
                };
                Response::from_json(&session)
            }
            Method::Put => {
                // Update session data
                let body = req.json::<serde_json::Value>().await?;

                let now = js_sys::Date::now() as u64;

                // If session doesn't exist, create it
                let created_at = match storage.get::<u64>("created_at").await {
                    Ok(t) => t,
                    Err(_) => now,
                };

                if let Some(user_id) = body.get("user_id").and_then(|v| v.as_str()) {
                    storage.put("user_id", user_id).await?;
                }

                if let Some(data) = body.get("data") {
                    storage.put("data", data).await?;
                }

                storage.put("created_at", created_at).await?;
                storage.put("updated_at", now).await?;

                Response::from_json(&serde_json::json!({
                    "status": "updated",
                    "user_id": body.get("user_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "timestamp": now
                }))
            }
            Method::Delete => {
                // Clear session
                storage.delete("user_id").await?;
                storage.delete("data").await?;
                storage.delete("created_at").await?;
                storage.delete("updated_at").await?;
                Response::ok("Session cleared")
            }
            _ => Response::error("Method not allowed", 405),
        }
    }
}
