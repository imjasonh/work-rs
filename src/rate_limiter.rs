//! Rate limiting implementation for R2 operations
//!
//! This module provides a token bucket-style rate limiter specifically designed
//! for Cloudflare R2's write limitations. R2 enforces a limit of 1 write per second
//! per object key to prevent conflicts and ensure consistency.
//!
//! # Architecture
//!
//! The rate limiter is implemented as a Durable Object to ensure consistent
//! rate limiting across all Worker instances globally. This prevents multiple
//! Workers from exceeding the rate limit for the same key.
//!
//! # Example
//!
//! ```rust
//! let mut limiter = RateLimiter::new(1); // 1 request per second
//!
//! // First request is allowed
//! assert!(limiter.check_rate_limit("file.txt").is_ok());
//!
//! // Second request within 1 second is rejected
//! match limiter.check_rate_limit("file.txt") {
//!     Err(retry_after) => {
//!         println!("Rate limited, retry after {:?}", retry_after);
//!     }
//!     Ok(_) => unreachable!(),
//! }
//! ```

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use worker::*;

/// Rate limiter for R2 write operations
///
/// This implements a sliding window rate limiter that tracks write attempts
/// per key and enforces the configured rate limit. The implementation is
/// optimized for Cloudflare R2's specific requirement of 1 write per second per key.
///
/// # Memory Management
///
/// The rate limiter includes automatic cleanup of old entries to prevent
/// unbounded memory growth. Call `cleanup()` periodically to remove expired entries.
pub struct RateLimiter {
    /// Map of object key to list of write timestamps (in milliseconds)
    write_history: HashMap<String, Vec<u64>>,
    /// Maximum writes per second per key
    max_writes_per_second: u32,
    /// Time window in milliseconds
    window_ms: u64,
}

impl RateLimiter {
    /// Creates a new rate limiter with the specified limit
    ///
    /// # Arguments
    ///
    /// * `max_writes_per_second` - Maximum number of writes allowed per second per key.
    ///   For R2, this should be set to 1.
    pub fn new(max_writes_per_second: u32) -> Self {
        Self {
            write_history: HashMap::new(),
            max_writes_per_second,
            window_ms: 1000, // 1 second window
        }
    }

    /// Check if a write is allowed for the given key
    ///
    /// This method implements a sliding window algorithm to track write attempts
    /// and enforce the rate limit. If the limit is exceeded, it returns the duration
    /// until the next write will be allowed.
    ///
    /// # Arguments
    ///
    /// * `key` - The R2 object key to check
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The write is allowed
    /// * `Err(Duration)` - The write is rate limited, with the duration until retry
    ///
    /// # Algorithm
    ///
    /// 1. Remove expired entries outside the time window
    /// 2. Check if we're at the limit
    /// 3. If at limit, calculate retry duration based on oldest entry
    /// 4. If not at limit, record the attempt and allow
    pub fn check_rate_limit(&mut self, key: &str) -> std::result::Result<(), Duration> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Get or create write history for this key
        let history = self.write_history.entry(key.to_string()).or_default();

        // Remove old entries outside the window
        let cutoff = now.saturating_sub(self.window_ms);
        history.retain(|&timestamp| timestamp > cutoff);

        // Check if we're at the limit
        if history.len() >= self.max_writes_per_second as usize {
            // Calculate when the oldest write will expire
            if let Some(&oldest) = history.first() {
                let retry_after_ms = oldest + self.window_ms - now;
                return Err(Duration::from_millis(retry_after_ms));
            }
        }

        // Add current timestamp and allow the write
        history.push(now);
        Ok(())
    }

    /// Clear old entries to prevent memory growth
    pub fn cleanup(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let cutoff = now.saturating_sub(self.window_ms * 2); // Keep 2 windows of history

        self.write_history.retain(|_, history| {
            history.retain(|&timestamp| timestamp > cutoff);
            !history.is_empty()
        });
    }
}

/// Create a rate limit error response with appropriate headers
///
/// This function creates a standardized 429 response with rate limit headers
/// that follow common API conventions.
///
/// # Arguments
///
/// * `retry_after` - Duration until the client should retry
///
/// # Headers Set
///
/// * `Retry-After` - Seconds until retry (decimal for precision)
/// * `X-RateLimit-Limit` - The rate limit (always 1 for R2)
/// * `X-RateLimit-Remaining` - Remaining requests (always 0 when rate limited)
/// * `X-RateLimit-Reset` - Unix timestamp when the limit resets
///
/// # Example
///
/// ```rust
/// let retry_duration = Duration::from_millis(500);
/// let response = rate_limit_response(retry_duration)?;
/// // Returns 429 with Retry-After: 0.5
/// ```
pub fn rate_limit_response(retry_after: Duration) -> Result<Response> {
    let seconds = retry_after.as_secs_f64();
    let headers = Headers::new();
    headers.set("Retry-After", &seconds.to_string())?;
    headers.set("X-RateLimit-Limit", "1")?;
    headers.set("X-RateLimit-Remaining", "0")?;
    headers.set(
        "X-RateLimit-Reset",
        &(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + retry_after.as_secs())
        .to_string(),
    )?;

    Ok(
        Response::error("Too Many Requests - R2 write rate limit exceeded", 429)?
            .with_headers(headers),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_first_write() {
        let mut limiter = RateLimiter::new(1);
        assert!(limiter.check_rate_limit("test.txt").is_ok());
    }

    #[test]
    fn test_rate_limiter_blocks_concurrent_writes() {
        let mut limiter = RateLimiter::new(1);

        // First write should succeed
        assert!(limiter.check_rate_limit("test.txt").is_ok());

        // Second write within the window should be blocked
        let result = limiter.check_rate_limit("test.txt");
        assert!(result.is_err());
        if let Err(duration) = result {
            assert!(duration.as_millis() > 0);
            assert!(duration.as_millis() <= 1000);
        }
    }

    #[test]
    fn test_rate_limiter_different_keys() {
        let mut limiter = RateLimiter::new(1);

        // Writes to different keys should not interfere
        assert!(limiter.check_rate_limit("file1.txt").is_ok());
        assert!(limiter.check_rate_limit("file2.txt").is_ok());
        assert!(limiter.check_rate_limit("file3.txt").is_ok());
    }

    #[test]
    fn test_cleanup_removes_old_entries() {
        let mut limiter = RateLimiter::new(1);

        // Add some entries
        limiter.check_rate_limit("test1.txt").ok();
        limiter.check_rate_limit("test2.txt").ok();

        // Manually set old timestamps
        if let Some(history) = limiter.write_history.get_mut("test1.txt") {
            history[0] = 0; // Very old timestamp
        }

        limiter.cleanup();

        // Old entry should be removed
        assert!(!limiter.write_history.contains_key("test1.txt"));
        assert!(limiter.write_history.contains_key("test2.txt"));
    }
}
