use wasm_bindgen::JsValue;
use worker::*;

mod counter_object;
mod r2_rate_limiter;
mod r2_storage;
mod rate_limiter;
mod session_object;

use r2_storage::handle_r2_request;

// Export Durable Objects
pub use counter_object::CounterObject;
pub use r2_rate_limiter::R2RateLimiterObject;
pub use session_object::SessionObject;

// Tests modules
#[cfg(test)]
mod counter_object_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod lib_tests;
#[cfg(test)]
mod r2_rate_limiter_tests;
#[cfg(test)]
mod r2_storage_tests;
#[cfg(test)]
mod session_object_tests;
#[cfg(test)]
mod tests;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let path = req.path();

    // Handle different routes without Router
    if path.starts_with("/files/") {
        // R2 operations
        match env.bucket("FILES_BUCKET") {
            Ok(bucket) => {
                let file_path = path.strip_prefix("/files/").unwrap_or("");
                handle_r2_request(req, bucket, file_path, &env).await
            }
            Err(_) => Response::error("R2 storage is not configured", 503),
        }
    } else if path.starts_with("/counter") {
        // Counter Durable Object operations
        handle_counter_request(req, env, &path).await
    } else if path.starts_with("/session") {
        // Session Durable Object operations
        handle_session_request(req, env, &path).await
    } else if path == "/" {
        // Root path
        Response::ok("Hello from Rust Workers! Available endpoints:\n/files/* - R2 operations\n/counter/* - Counter operations\n/session/* - Session operations")
    } else {
        Response::error("Not found", 404)
    }
}

async fn handle_counter_request(req: Request, env: Env, path: &str) -> Result<Response> {
    // Get the counter ID from the path
    let counter_id = if path == "/counter" || path == "/counter/" {
        "default"
    } else {
        path.strip_prefix("/counter/").unwrap_or("default")
    };

    // Get the Durable Object namespace
    let namespace = match env.durable_object("COUNTER_OBJECT") {
        Ok(ns) => ns,
        Err(_) => return Response::error("Counter service not available", 503),
    };

    // Get the Durable Object stub
    let id = namespace.id_from_name(counter_id)?;
    let stub = id.get_stub()?;

    // Forward the request to the Durable Object
    match req.method() {
        Method::Get => {
            let mut response = stub.fetch_with_str("https://fake-host/").await?;
            Response::from_json(&response.json::<serde_json::Value>().await?)
        }
        Method::Post => {
            let request = Request::new_with_init(
                "https://fake-host/increment",
                RequestInit::new().with_method(Method::Post),
            )?;
            let mut response = stub.fetch_with_request(request).await?;
            Response::from_json(&response.json::<serde_json::Value>().await?)
        }
        Method::Delete => {
            let request = Request::new_with_init(
                "https://fake-host/",
                RequestInit::new().with_method(Method::Delete),
            )?;
            let response = stub.fetch_with_request(request).await?;
            Ok(response)
        }
        _ => Response::error("Method not allowed", 405),
    }
}

async fn handle_session_request(mut req: Request, env: Env, path: &str) -> Result<Response> {
    // Get the session ID from the path
    let parts: Vec<&str> = path
        .strip_prefix("/session/")
        .unwrap_or("")
        .split('/')
        .collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Response::error("Session ID required", 400);
    }

    let session_id = parts[0];
    let key = parts.get(1).copied();

    // Get the Durable Object namespace
    let namespace = match env.durable_object("SESSION_OBJECT") {
        Ok(ns) => ns,
        Err(_) => return Response::error("Session service not available", 503),
    };

    // Get the Durable Object stub
    let id = namespace.id_from_name(session_id)?;
    let stub = id.get_stub()?;

    // Build the request path for the Durable Object
    let do_path = match key {
        Some(k) => format!("/{}", k),
        None => "/".to_string(),
    };

    // Forward the request to the Durable Object with the body if present
    let mut response = match req.method() {
        Method::Put => {
            let body = req.text().await?;
            let headers = Headers::new();
            headers.set("content-type", "application/json")?;
            let request = Request::new_with_init(
                &format!("https://fake-host{}", do_path),
                RequestInit::new()
                    .with_method(Method::Put)
                    .with_body(Some(JsValue::from_str(&body)))
                    .with_headers(headers),
            )?;
            stub.fetch_with_request(request).await?
        }
        Method::Get => {
            let request = Request::new_with_init(
                &format!("https://fake-host{}", do_path),
                RequestInit::new().with_method(Method::Get),
            )?;
            stub.fetch_with_request(request).await?
        }
        Method::Delete => {
            let request = Request::new_with_init(
                &format!("https://fake-host{}", do_path),
                RequestInit::new().with_method(Method::Delete),
            )?;
            stub.fetch_with_request(request).await?
        }
        _ => return Response::error("Method not allowed", 405),
    };

    // Return the response
    match req.method() {
        Method::Delete => Ok(response), // DELETE returns plain text
        _ => {
            // Check if response is an error
            if response.status_code() >= 400 {
                Ok(response)
            } else {
                let json = response.json::<serde_json::Value>().await?;
                Response::from_json(&json)
            }
        }
    }
}
