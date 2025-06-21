# work-rs

A simple Rust web server that runs on Cloudflare Workers using WebAssembly.

## Overview

This project demonstrates how to build a Rust application for Cloudflare Workers using the `workers-rs` SDK. The application compiles to WebAssembly and runs at Cloudflare's edge locations globally.

## Features

- 🦀 Written in 100% Rust
- 🌐 Runs on Cloudflare's global edge network
- ⚡ Fast WebAssembly execution
- 🔧 HTTP routing with multiple endpoints
- 📦 JSON serialization/deserialization with Serde
- 💾 R2 object storage integration
- 🔄 Durable Objects for stateful services
- ✅ Unit tests with mocks

## API Endpoints

### Basic Endpoints

#### GET /
Returns a simple greeting message.

**Response:**
```
Hello from Rust Workers!
```

#### POST /api/data
Accepts JSON data and echoes it back with a timestamp.

**Request:**
```json
{
  "message": "Your message here"
}
```

**Response:**
```json
{
  "message": "Echo: Your message here",
  "timestamp": 1699564800000
}
```

### R2 Storage Endpoints

#### GET /files/
List all files in the R2 bucket.

**Response:**
```json
["file1.txt", "images/photo.jpg"]
```

#### GET /files/{path}
Download a specific file from R2.

**Response:** Binary file content with appropriate Content-Type header

#### PUT /files/{path}
Upload a file to R2.

**Request:** Binary file data in request body

**Response:**
```json
{
  "key": "path/to/file.txt",
  "size": 1234,
  "content_type": "text/plain",
  "uploaded_at": 1699564800000
}
```

#### DELETE /files/{path}
Delete a file from R2.

**Response:** `File deleted`

### Durable Objects Endpoints

#### Counter Object

##### GET /counter/
Get current counter value.

**Response:**
```json
{
  "count": 42,
  "last_updated": 1699564800000
}
```

##### POST /counter/increment
Increment the counter.

**Response:**
```json
{
  "count": 43,
  "last_updated": 1699564800000
}
```

##### POST /counter/decrement
Decrement the counter.

**Response:**
```json
{
  "count": 41,
  "last_updated": 1699564800000
}
```

##### DELETE /counter/
Reset the counter.

**Response:** `Counter reset`

#### Session Object

##### GET /session/{session_id}/
Get session data.

**Response:**
```json
{
  "user_id": "user123",
  "data": {
    "preferences": {
      "theme": "dark"
    }
  },
  "created_at": 1699564800000,
  "updated_at": 1699564800000
}
```

##### PUT /session/{session_id}/
Update session data.

**Request:**
```json
{
  "user_id": "user123",
  "data": {
    "preferences": {
      "theme": "light"
    }
  }
}
```

**Response:** `Session updated`

##### DELETE /session/{session_id}/
Clear session data.

**Response:** `Session cleared`

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Node.js](https://nodejs.org/) (for wrangler CLI)
- [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/): `npm install -g wrangler`

## Development

1. **Install dependencies:**
   ```bash
   cargo build
   ```

2. **Run tests:**
   ```bash
   cargo test
   ```

3. **Run locally:**
   ```bash
   wrangler dev
   ```
   
   The server will start at `http://localhost:8787`

4. **Test the endpoints:**
   ```bash
   # Test GET endpoint
   curl http://localhost:8787/
   
   # Test POST endpoint
   curl -X POST http://localhost:8787/api/data \
     -H "Content-Type: application/json" \
     -d '{"message": "Hello from Rust!"}'
   
   # Test R2 upload
   curl -X PUT http://localhost:8787/files/test.txt \
     -H "Content-Type: text/plain" \
     -d "Hello, R2!"
   
   # Test counter
   curl -X POST http://localhost:8787/counter/increment
   ```

## Deployment

1. **Login to Cloudflare (first time only):**
   ```bash
   wrangler login
   ```

2. **Deploy to Cloudflare Workers:**
   ```bash
   wrangler deploy
   ```

## Project Structure

```
.
├── Cargo.toml          # Rust dependencies and configuration
├── wrangler.toml       # Cloudflare Workers configuration
├── src/
│   ├── lib.rs         # Main application code
│   ├── counter_object.rs  # Counter Durable Object
│   ├── session_object.rs  # Session Durable Object
│   ├── r2_storage.rs      # R2 storage operations
│   └── tests.rs           # Unit tests
└── build/             # Generated WebAssembly output (after building)
```

## Configuration

The `wrangler.toml` file contains the Cloudflare Workers configuration:
- `name`: The name of your Worker
- `main`: Entry point for the Worker
- `compatibility_date`: Cloudflare compatibility date
- `build.command`: Command to build the Rust project to WebAssembly
- `r2_buckets`: R2 bucket bindings for object storage
- `durable_objects.bindings`: Durable Object namespaces
- `migrations`: Durable Object class migrations

### Setting up R2 and Durable Objects

Before deploying, you need to create an R2 bucket:
```bash
wrangler r2 bucket create work-rs-files
```

Durable Objects will be automatically created on first deployment.

## How It Works

1. The Rust code is compiled to WebAssembly using `wasm32-unknown-unknown` target
2. The `worker-build` tool packages the WebAssembly module with JavaScript glue code
3. Cloudflare Workers runtime executes the WebAssembly module in V8 isolates at the edge

## Dependencies

- `worker` (v0.4.2): Cloudflare Workers Rust SDK
- `serde` (v1.0): Serialization/deserialization framework
- `serde_json` (v1.0): JSON support for Serde
- `async-trait` (v0.1): Async trait support

### Dev Dependencies
- `tokio` (v1): Async runtime for tests
- `mockall` (v0.11): Mocking framework for unit tests

## Resources

- [Cloudflare Workers Documentation](https://developers.cloudflare.com/workers/)
- [workers-rs GitHub Repository](https://github.com/cloudflare/workers-rs)
- [Rust on Cloudflare Workers Guide](https://developers.cloudflare.com/workers/languages/rust/)

## License

This project is open source and available under the MIT License.