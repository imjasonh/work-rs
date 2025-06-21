use worker::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CounterData {
    pub count: i32,
    pub last_updated: u64,
}

/// A simple counter Durable Object that maintains state across requests
#[durable_object]
pub struct CounterObject {
    state: State,
    env: Env,
}

#[durable_object]
impl DurableObject for CounterObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&mut self, _req: Request) -> Result<Response> {
        let mut storage = self.state.storage();
        let path = _req.path();

        match _req.method() {
            Method::Get => {
                let count: Option<i32> = storage.get("count").await?;
                let count = count.unwrap_or(0);
                let last_updated: Option<u64> = storage.get("last_updated").await?;
                let last_updated = last_updated.unwrap_or(0);
                
                let data = CounterData { count, last_updated };
                Response::from_json(&data)
            }
            Method::Post => {
                if path == "/increment" {
                    let count: Option<i32> = storage.get("count").await?;
                    let mut count = count.unwrap_or(0);
                    count += 1;
                    
                    storage.put("count", count).await?;
                    storage.put("last_updated", js_sys::Date::now() as u64).await?;
                    
                    let data = CounterData {
                        count,
                        last_updated: js_sys::Date::now() as u64,
                    };
                    Response::from_json(&data)
                } else if path == "/decrement" {
                    let count: Option<i32> = storage.get("count").await?;
                    let mut count = count.unwrap_or(0);
                    count -= 1;
                    
                    storage.put("count", count).await?;
                    storage.put("last_updated", js_sys::Date::now() as u64).await?;
                    
                    let data = CounterData {
                        count,
                        last_updated: js_sys::Date::now() as u64,
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
            _ => Response::error("Method not allowed", 405)
        }
    }
}