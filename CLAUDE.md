# Cloudflare Workers with Rust: Key Learnings

This document captures important insights and patterns discovered while building a Rust-based Cloudflare Worker with Durable Objects and R2 storage.

## Core Concepts

### WebAssembly Compilation
- Cloudflare Workers run WebAssembly (WASM) modules in V8 isolates
- Rust compiles to WASM using the `wasm32-unknown-unknown` target
- The `worker-build` tool handles bundling and JavaScript glue code generation

### workers-rs SDK
The `workers-rs` crate provides Rust bindings for the Cloudflare Workers runtime API:
- Uses `#[event(fetch)]` macro to define the main entry point
- Provides Request/Response types that mirror the Fetch API
- Offers bindings for platform features like KV, R2, and Durable Objects

## Durable Objects

### Key Characteristics
1. **Single-threaded consistency**: Each Durable Object ID maps to exactly one instance globally
2. **Automatic migration**: Objects migrate between data centers to be close to users
3. **Transactional storage**: Built-in key-value storage with ACID guarantees
4. **WebSocket support**: Can maintain long-lived connections

### Implementation Pattern
```rust
#[durable_object]
pub struct MyObject {
    state: State,
    env: Env,
}

#[durable_object]
impl DurableObject for MyObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        // Handle HTTP requests to the Durable Object
    }
}
```

### Common Pitfalls and Solutions

1. **Routing to Durable Objects**
   - Problem: Direct routing with workers-rs Router doesn't work well with Durable Objects
   - Solution: Use manual path matching in the main fetch handler
   ```rust
   if path.starts_with("/counter") {
       handle_counter_request(req, env, &path).await
   }
   ```

2. **Request Forwarding**
   - Problem: Can't directly forward requests to Durable Objects
   - Solution: Create new requests with fake hosts
   ```rust
   let request = Request::new_with_init(
       "https://fake-host/path",
       RequestInit::new().with_method(method)
   )?;
   stub.fetch_with_request(request).await
   ```

3. **Storage Operations**
   - Always use `put_multiple` for batch operations
   - Remember that storage is eventually consistent outside transactions
   - Use `transaction` for atomic operations

## R2 Storage Integration

### Key Features
- S3-compatible API without egress fees
- Integrated with Workers through bindings
- Supports streaming for large files

### Implementation Patterns
```rust
// List objects
let objects = bucket.list()
    .prefix(prefix)
    .delimiter("/")
    .execute()
    .await?;

// Stream file uploads
let body = req.stream()?;
bucket.put(key, body)
    .http_metadata(metadata)
    .execute()
    .await?;
```

### Best Practices
1. Use streaming for large files to avoid memory limits
2. Set appropriate Content-Type headers for served files
3. Implement proper error handling for missing objects
4. Use conditional requests (ETags) for caching

## Testing Strategies

### Unit Testing Challenges
- Workers runtime APIs aren't available in standard Rust tests
- Durable Objects require runtime infrastructure
- Async trait implementations complicate mocking

### Solutions
1. **Abstract business logic**: Separate pure functions from runtime dependencies
2. **Mock storage layer**: Create simple HashMap-based mocks for testing logic
3. **Integration tests**: Use `wrangler dev` for end-to-end testing

Example mock pattern:
```rust
#[cfg(test)]
struct MockKVStore {
    data: HashMap<String, serde_json::Value>,
}
```

## Performance Considerations

1. **Cold Starts**:
   - Workers have minimal cold start times (~5ms)
   - Rust WASM modules are larger than JavaScript but still performant

2. **Memory Limits**:
   - Workers have 128MB memory limit
   - Stream large files instead of loading into memory
   - Be mindful of string allocations

3. **CPU Limits**:
   - 10ms CPU time for free tier, 30s for paid
   - Durable Objects have 30s limit
   - Use `ctx.wait_until()` for background tasks

## Development Workflow

### Local Development
```bash
# Run with local Durable Objects persistence
wrangler dev --local --persist-to=/tmp/durable-objects

# Tail production logs
wrangler tail
```

### Debugging Tips
1. Use `console_log!` macro for logging (appears in wrangler logs)
2. Return detailed error messages during development
3. Use `wrangler dev` with `--local` flag for faster iteration

### Deployment Best Practices
1. Test Durable Object migrations in staging first
2. Use compatibility dates to control platform changes
3. Monitor with `wrangler tail` after deployment
4. Set up proper error alerting

## Common Gotchas

1. **Async Trait Bounds**: The `#[durable_object]` macro has specific requirements for async functions
2. **Path Handling**: Always handle trailing slashes in routes
3. **Error Types**: Must use `worker::Error` type throughout
4. **JSON Serialization**: Ensure all types implement Serialize/Deserialize
5. **Request Body**: Can only be read once - clone if needed multiple times

## Useful Patterns

### Request Validation
```rust
let data: MyType = match req.json().await {
    Ok(data) => data,
    Err(_) => return Response::error("Invalid JSON", 400)
};
```

### CORS Handling
```rust
Response::ok("data")?
    .with_headers(headers! {
        "Access-Control-Allow-Origin" => "*",
        "Access-Control-Allow-Methods" => "GET, POST, OPTIONS"
    })
```

### Error Responses
```rust
fn error_response(msg: &str, status: u16) -> Result<Response> {
    Response::error(msg, status)
        .or_else(|_| Response::ok(msg)?.with_status(status))
}
```

## Resources for Deep Dives

1. **Durable Objects**: Understanding the consistency model and limits
2. **WebAssembly**: How Rust compiles and optimizations
3. **Edge Computing**: Distributed systems challenges at the edge
4. **workers-rs Source**: The SDK source code is well-documented

## Rate Limiting Implementation

### R2 Write Rate Limits
- R2 enforces a limit of **1 write per second per object key**
- Exceeding this limit results in HTTP 429 responses
- This is a hard limit that cannot be increased

### Rate Limiting Architecture
We implemented a Durable Object-based rate limiter to handle this globally:

1. **RateLimiter** (`rate_limiter.rs`): Core rate limiting logic using sliding window algorithm
2. **R2RateLimiterObject** (`r2_rate_limiter.rs`): Durable Object that maintains global state
3. **Integration** in `r2_storage.rs`: Checks rate limit before attempting R2 writes

### Key Design Decisions
- **Durable Object for Global State**: Ensures rate limiting works across all Worker instances
- **Sliding Window Algorithm**: Accurate rate limiting with millisecond precision
- **RefCell for Interior Mutability**: Required because DurableObject methods take &self
- **Automatic Cleanup**: Prevents unbounded memory growth

### API Response Format
When rate limited, the API returns:
```
HTTP/1.1 429 Too Many Requests
Retry-After: 0.75
X-RateLimit-Limit: 1
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1699564801
```

### Testing Considerations
- Unit tests can't test worker-specific APIs (Headers, Response)
- Use `test-rate-limit.sh` script for integration testing
- Load testing revealed the need for this rate limiting

## Future Considerations

- WebAssembly Component Model support
- Improved debugging tools for WASM
- Native async trait support in Rust
- Expanded Durable Objects features (indexes, queries)
- Rate limiter sharding for high-volume scenarios
