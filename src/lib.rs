use worker::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ApiDataRequest {
    message: String,
}

#[derive(Serialize)]
struct ApiDataResponse {
    message: String,
    timestamp: u64,
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_log!("Request received: {} {}", req.method(), req.path());
    
    let router = Router::new();
    
    router
        .get("/", |_, _| Response::ok("Hello from Rust Workers!"))
        .post_async("/api/data", |mut req, _| async move {
            let data: ApiDataRequest = req.json().await?;
            console_log!("Received data: {}", data.message);
            
            let response = ApiDataResponse {
                message: format!("Echo: {}", data.message),
                timestamp: js_sys::Date::now() as u64,
            };
            
            Response::from_json(&response)
        })
        .run(req, env)
        .await
}