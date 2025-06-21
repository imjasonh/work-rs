# work-rs

A simple Rust web server that runs on Cloudflare Workers using WebAssembly.

## Overview

This project demonstrates how to build a Rust application for Cloudflare Workers using the `workers-rs` SDK. The application compiles to WebAssembly and runs at Cloudflare's edge locations globally.

## Features

- 🦀 Written in 100% Rust
- 🌐 Runs on Cloudflare's global edge network
- ⚡ Fast WebAssembly execution
- 🔧 Simple HTTP routing with GET and POST endpoints
- 📦 JSON serialization/deserialization with Serde

## API Endpoints

### GET /
Returns a simple greeting message.

**Response:**
```
Hello from Rust Workers!
```

### POST /api/data
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

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Node.js](https://nodejs.org/) (for wrangler CLI)
- [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/): `npm install -g wrangler`

## Development

1. **Install dependencies:**
   ```bash
   cargo build
   ```

2. **Run locally:**
   ```bash
   wrangler dev
   ```
   
   The server will start at `http://localhost:8787`

3. **Test the endpoints:**
   ```bash
   # Test GET endpoint
   curl http://localhost:8787/
   
   # Test POST endpoint
   curl -X POST http://localhost:8787/api/data \
     -H "Content-Type: application/json" \
     -d '{"message": "Hello from Rust!"}'
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
│   └── lib.rs         # Main application code
└── build/             # Generated WebAssembly output (after building)
```

## Configuration

The `wrangler.toml` file contains the Cloudflare Workers configuration:
- `name`: The name of your Worker
- `main`: Entry point for the Worker
- `compatibility_date`: Cloudflare compatibility date
- `build.command`: Command to build the Rust project to WebAssembly

## How It Works

1. The Rust code is compiled to WebAssembly using `wasm32-unknown-unknown` target
2. The `worker-build` tool packages the WebAssembly module with JavaScript glue code
3. Cloudflare Workers runtime executes the WebAssembly module in V8 isolates at the edge

## Dependencies

- `worker` (v0.4.2): Cloudflare Workers Rust SDK
- `serde` (v1.0): Serialization/deserialization framework
- `serde_json` (v1.0): JSON support for Serde

## Resources

- [Cloudflare Workers Documentation](https://developers.cloudflare.com/workers/)
- [workers-rs GitHub Repository](https://github.com/cloudflare/workers-rs)
- [Rust on Cloudflare Workers Guide](https://developers.cloudflare.com/workers/languages/rust/)

## License

This project is open source and available under the MIT License.