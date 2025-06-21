use worker::*;

mod r2_storage;
use r2_storage::handle_r2_request;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let path = req.path();
    
    if path.starts_with("/files/") {
        match env.bucket("FILES_BUCKET") {
            Ok(bucket) => {
                let file_path = path.strip_prefix("/files/").unwrap_or("");
                handle_r2_request(req, bucket, file_path).await
            },
            Err(_) => Response::error("R2 storage is not configured", 503)
        }
    } else {
        Response::ok("Hello from Rust Workers! Use /files/* for R2 operations.")
    }
}