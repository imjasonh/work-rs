use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize, Deserialize)]
pub struct CounterData {
    pub count: i32,
    pub last_updated: u64,
}

/// A simple counter Durable Object that maintains state across requests
#[durable_object]
pub struct CounterObject {
    state: State,
    _env: Env,
}

impl DurableObject for CounterObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, _env: env }
    }

    async fn fetch(&self, _req: Request) -> Result<Response> {
        let storage = self.state.storage();
        let path = _req.path();

        match _req.method() {
            Method::Get => {
                let count = storage.get::<i32>("count").await.unwrap_or_default();
                let last_updated = storage.get::<u64>("last_updated").await.unwrap_or_default();

                let data = CounterData {
                    count,
                    last_updated,
                };
                Response::from_json(&data)
            }
            Method::Post => {
                if path.ends_with("/increment") {
                    let mut count = storage.get::<i32>("count").await.unwrap_or_default();
                    count += 1;

                    let now = js_sys::Date::now() as u64;
                    storage.put("count", count).await?;
                    storage.put("last_updated", now).await?;

                    let data = CounterData {
                        count,
                        last_updated: now,
                    };
                    Response::from_json(&data)
                } else if path.ends_with("/decrement") {
                    let mut count = storage.get::<i32>("count").await.unwrap_or_default();
                    count -= 1;

                    let now = js_sys::Date::now() as u64;
                    storage.put("count", count).await?;
                    storage.put("last_updated", now).await?;

                    let data = CounterData {
                        count,
                        last_updated: now,
                    };
                    Response::from_json(&data)
                } else {
                    Response::error("Invalid path", 404)
                }
            }
            Method::Delete => {
                storage.delete("count").await?;
                storage.delete("last_updated").await?;
                Response::ok("Counter reset")
            }
            _ => Response::error("Method not allowed", 405),
        }
    }
}
