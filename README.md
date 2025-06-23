# work-rs

[![CI](https://github.com/imjasonh/work-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/imjasonh/work-rs/actions/workflows/ci.yml)

A Rust web server that runs on Cloudflare Workers using WebAssembly, featuring Durable Objects for stateful services and R2 for object storage.

## Overview

This project demonstrates how to build a Rust application for Cloudflare Workers using the `workers-rs` SDK. The application compiles to WebAssembly and runs at Cloudflare's edge locations globally.

## Features

- 🦀 Written in 100% Rust
- 🌐 Runs on Cloudflare's global edge network
- ⚡ Fast WebAssembly execution
- 🔧 HTTP routing with multiple endpoints
- 📦 JSON serialization/deserialization with Serde
- 💾 R2 object storage with content-addressable storage (CAS)
- 🔄 Durable Objects for stateful services
- ✅ Unit tests with mocks
- 🚀 CI/CD with GitHub Actions and pre-commit hooks
- 🎯 Automatic content deduplication to avoid R2 rate limits

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

### R2 Storage Endpoints (with Content-Addressable Storage)

The R2 storage implementation uses content-addressable storage (CAS) to automatically deduplicate files and avoid R2's rate limits. Files with identical content are stored only once, regardless of their filenames.

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
Upload a file to R2. The system automatically:
- Computes SHA256 hash of the content
- Stores the content in `blobs/{sha256}` if it doesn't already exist
- Maps the filename to the content hash
- Returns 304 Not Modified if re-uploading identical content

**Request:** Binary file data in request body

**Response:**
```json
{
  "key": "path/to/file.txt",
  "size": 1234,
  "content_type": "text/plain",
  "uploaded_at": 1699564800000,
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}
```

#### DELETE /files/{path}
Delete a file mapping from R2. Note: The actual blob content is preserved as it may be referenced by other files.

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
- [Node.js](https://nodejs.org/) (v16 or later, for wrangler CLI)
- [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/): `npm install -g wrangler`
- A Cloudflare account (free tier is sufficient for development)

## Development

1. **Clone the repository:**
   ```bash
   git clone https://github.com/imjasonh/work-rs.git
   cd work-rs
   ```

2. **Install dependencies:**
   ```bash
   cargo build
   ```

3. **Set up pre-commit hooks (optional but recommended):**
   ```bash
   # Install pre-commit (if not already installed)
   brew install pre-commit  # or pip install pre-commit

   # Install the git hooks
   pre-commit install
   ```

4. **Run tests:**
   ```bash
   cargo test
   ```

5. **Run locally with Durable Objects persistence:**
   ```bash
   wrangler dev --local --persist-to=/tmp/durable-objects
   ```

   The server will start at `http://localhost:8787`

   Note: The `--local` flag runs the worker locally, and `--persist-to` enables Durable Objects persistence during development.

6. **Test the endpoints:**
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

7. **Run end-to-end tests:**
   ```bash
   # Test against local development server
   ./e2e-test.sh http://localhost:8787

   # Test against deployed Worker
   ./e2e-test.sh https://your-worker.workers.dev
   ```

## Testing

### Unit Tests
Run Rust unit tests with mocks:
```bash
cargo test
```

### End-to-End Tests
The `e2e-test.sh` script runs comprehensive tests against a live Worker:
- Basic connectivity and routing
- Counter Durable Object operations
- Session storage functionality
- R2 file upload/download/delete
- Security tests (path traversal prevention)

E2E tests run automatically:
- Against preview deployments on PRs
- Against production after merging to main

## Deployment

### Manual Deployment

1. **Login to Cloudflare (first time only):**
   ```bash
   wrangler login
   ```

2. **Create R2 bucket (first time only):**
   ```bash
   wrangler r2 bucket create work-rs-files
   ```

3. **Configure R2 lifecycle rules (optional):**
   ```bash
   # Delete objects after 1 day (useful for temporary/test data)
   wrangler r2 bucket lifecycle add work-rs-files "delete-after-1-day" --expire-days 1

   # List all lifecycle rules
   wrangler r2 bucket lifecycle list work-rs-files
   ```

4. **Deploy to Cloudflare Workers:**
   ```bash
   wrangler deploy
   ```

   This will compile your Rust code to WebAssembly, bundle it, and deploy it to Cloudflare's edge network.

### Automated Deployment (GitHub Actions)

This repository includes a GitHub Actions workflow that automatically deploys to Cloudflare Workers on pushes to the `main` branch.

#### Setup

1. **Get your Cloudflare API token:**
   - Go to [Cloudflare Dashboard](https://dash.cloudflare.com/profile/api-tokens)
   - Click "Create Token"
   - Use the "Edit Cloudflare Workers" template or create a custom token with:
     - Account: `Cloudflare Workers Scripts:Edit`
     - Zone: `Zone:Read` (if using custom domains)

2. **Get your Cloudflare Account ID:**
   - Go to any domain in your Cloudflare account
   - The Account ID is in the right sidebar

3. **Add secrets to your GitHub repository:**
   - Go to your repository on GitHub
   - Navigate to Settings → Secrets and variables → Actions
   - Add the following secrets:
     - `CLOUDFLARE_API_TOKEN`: Your API token from step 1
     - `CLOUDFLARE_ACCOUNT_ID`: Your account ID from step 2

Once configured, every push to the `main` branch will automatically build and deploy your Worker to Cloudflare.

### Preview Deployments

Pull requests automatically get preview deployments after CI passes. Each PR deploys to a separate Worker with its own Durable Objects.

#### Features
- Automatic deployment after CI/tests pass
- Isolated Durable Objects (separate from production)
- Automatic cleanup when PR is closed
- Comment on PR with preview URL

#### Setup
Add these additional secrets (optional):
- `CF_ACCOUNT_SUBDOMAIN`: Your Cloudflare subdomain (defaults to 'workers')

Preview URLs follow the pattern: `https://work-rs-preview-{PR_NUMBER}.{SUBDOMAIN}.dev`

**Note**: Preview deployments create separate Workers with their own Durable Objects, so data is not shared with production.

## Project Structure

```
.
├── Cargo.toml          # Rust dependencies and configuration
├── wrangler.toml       # Cloudflare Workers configuration
├── src/
│   ├── lib.rs         # Main application code
│   ├── counter_object.rs  # Counter Durable Object
│   ├── session_object.rs  # Session Durable Object
│   ├── file_mapping_object.rs  # File mapping Durable Object for CAS
│   ├── r2_storage.rs      # R2 storage operations with CAS
│   ├── sha256.rs          # SHA256 hashing utilities
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

### Environment Configuration

The `wrangler.toml` file includes bindings for:
- **R2 Storage**: The `FILES_BUCKET` binding connects to your R2 bucket
- **Durable Objects**:
  - `COUNTER_OBJECT`: Counter service for increment/decrement operations
  - `SESSION_OBJECT`: Session storage for user data
  - `FILE_MAPPING_OBJECT`: Filename to SHA256 mapping for content-addressable storage

Durable Objects are automatically provisioned on first deployment and will be available globally.

## How It Works

1. **Compilation**: The Rust code is compiled to WebAssembly using the `wasm32-unknown-unknown` target
2. **Bundling**: The `worker-build` tool packages the WebAssembly module with JavaScript glue code
3. **Execution**: Cloudflare Workers runtime executes the WebAssembly module in V8 isolates at edge locations
4. **Routing**: The main `fetch` handler routes requests to appropriate handlers based on URL paths
5. **State Management**: Durable Objects provide consistent, low-latency storage for stateful operations
6. **Object Storage**: R2 provides S3-compatible object storage without egress fees
7. **Content-Addressable Storage**: Files are stored by their SHA256 hash to avoid duplicate writes and R2 rate limits

## Dependencies

- `worker` (v0.4.2): Cloudflare Workers Rust SDK
- `serde` (v1.0): Serialization/deserialization framework
- `serde_json` (v1.0): JSON support for Serde
- `async-trait` (v0.1): Async trait support

### Dev Dependencies
- `tokio` (v1): Async runtime for tests
- `mockall` (v0.11): Mocking framework for unit tests

## Content-Addressable Storage

This implementation uses content-addressable storage (CAS) to solve R2's rate limit issues (1 write per second per key). Here's how it works:

1. **SHA256 Hashing**: When a file is uploaded, its SHA256 hash is computed using the Web Crypto API
2. **Blob Storage**: Content is stored in R2 at `blobs/{sha256}` instead of the original filename
3. **Conditional Writes**: Before writing, we check if the blob already exists to avoid duplicate writes
4. **Filename Mapping**: A Durable Object (`FileMappingObject`) maintains the mapping between filenames and content hashes
5. **Deduplication**: Multiple files with identical content share the same blob, dramatically reducing R2 writes

### Benefits

- **No Rate Limit Errors**: Load tests show 0% error rate (down from 56.5%)
- **Storage Efficiency**: Duplicate content is stored only once
- **Fast Uploads**: 304 Not Modified responses for re-uploaded content
- **Scalability**: Can handle many more concurrent users without hitting R2 limits

## Troubleshooting

### Common Issues

1. **"R2 storage is not configured" error**
   - Ensure you've created the R2 bucket: `wrangler r2 bucket create work-rs-files`
   - Check that your `wrangler.toml` has the correct bucket binding

2. **Durable Objects not working locally**
   - Use the `--local` flag with wrangler dev
   - Add `--persist-to=/tmp/durable-objects` for persistence between restarts

3. **Build errors**
   - Ensure you have the correct Rust toolchain: `rustup target add wasm32-unknown-unknown`
   - Try clearing the build cache: `cargo clean && cargo build`

## Resources

- [Cloudflare Workers Documentation](https://developers.cloudflare.com/workers/)
- [workers-rs GitHub Repository](https://github.com/cloudflare/workers-rs)
- [Rust on Cloudflare Workers Guide](https://developers.cloudflare.com/workers/languages/rust/)
- [Durable Objects Documentation](https://developers.cloudflare.com/durable-objects/)
- [R2 Storage Documentation](https://developers.cloudflare.com/r2/)

## License

This project is open source and available under the MIT License.
