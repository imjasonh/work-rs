use worker::*;
use serde::{Deserialize, Serialize};

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
    env: Env,
}

#[durable_object]
impl DurableObject for SessionObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&mut self, mut req: Request) -> Result<Response> {
        let mut storage = self.state.storage();

        match req.method() {
            Method::Get => {
                // Get all session data
                let user_id: Option<String> = storage.get("user_id").await?;
                let data: Option<serde_json::Value> = storage.get("data").await?;
                let created_at: Option<u64> = storage.get("created_at").await?;
                let updated_at: Option<u64> = storage.get("updated_at").await?;

                if let (Some(user_id), Some(data), Some(created_at), Some(updated_at)) = 
                    (user_id, data, created_at, updated_at) {
                    let session = SessionData {
                        user_id,
                        data,
                        created_at,
                        updated_at,
                    };
                    Response::from_json(&session)
                } else {
                    Response::error("Session not found", 404)
                }
            }
            Method::Put => {
                // Update session data
                let body = req.json::<serde_json::Value>().await?;
                
                let now = js_sys::Date::now() as u64;
                
                // If session doesn't exist, create it
                let created_at: Option<u64> = storage.get("created_at").await?;
                let created_at = created_at.unwrap_or(now);
                
                if let Some(user_id) = body.get("user_id").and_then(|v| v.as_str()) {
                    storage.put("user_id", user_id).await?;
                }
                
                if let Some(data) = body.get("data") {
                    storage.put("data", data).await?;
                }
                
                storage.put("created_at", created_at).await?;
                storage.put("updated_at", now).await?;
                
                Response::ok("Session updated")
            }
            Method::Delete => {
                // Clear session
                storage.delete("user_id").await?;
                storage.delete("data").await?;
                storage.delete("created_at").await?;
                storage.delete("updated_at").await?;
                Response::ok("Session cleared")
            }
            _ => Response::error("Method not allowed", 405)
        }
    }
}