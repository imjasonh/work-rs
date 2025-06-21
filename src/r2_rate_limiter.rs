//! Durable Object implementation for R2 rate limiting
//!
//! This module provides a Durable Object that maintains global rate limiting state
//! for R2 operations across all Worker instances. This ensures that the rate limit
//! is enforced consistently regardless of which Worker instance handles the request.

use crate::rate_limiter::{rate_limit_response, RateLimiter};
use std::cell::RefCell;
use worker::*;

/// Durable Object that maintains rate limiting state for R2 operations
///
/// This Durable Object acts as a centralized rate limiter for R2 write operations.
/// It ensures that the 1 write per second per key limit is enforced globally across
/// all Worker instances, preventing rate limit violations when multiple Workers
/// attempt to write to the same key.
///
/// # Design Decisions
///
/// - Uses a singleton pattern (single global instance) for simplicity
/// - Could be sharded by key prefix for better scalability in high-volume scenarios
/// - Uses RefCell for interior mutability since DurableObject methods take &self
///
/// # API
///
/// The Durable Object exposes a simple HTTP API:
/// - `GET /check/{key}` - Check if a write is allowed for the given key
///   - Returns 200 if allowed
///   - Returns 429 with Retry-After header if rate limited
#[durable_object]
pub struct R2RateLimiterObject {
    _state: State,
    _env: Env,
    limiter: RefCell<RateLimiter>,
}

impl DurableObject for R2RateLimiterObject {
    fn new(state: State, env: Env) -> Self {
        Self {
            _state: state,
            _env: env,
            // R2 allows 1 write per second per key
            limiter: RefCell::new(RateLimiter::new(1)),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let path = req.path();

        // Extract the operation and key from the path
        // Expected format: /check/{key}
        if !path.starts_with("/check/") {
            return Response::error("Invalid path", 400);
        }

        let key = path.strip_prefix("/check/").unwrap_or("");
        if key.is_empty() {
            return Response::error("Key is required", 400);
        }

        // Periodically cleanup old entries
        self.limiter.borrow_mut().cleanup();

        // Check rate limit
        match self.limiter.borrow_mut().check_rate_limit(key) {
            Ok(()) => {
                // Write is allowed
                Response::ok("allowed")
            }
            Err(retry_after) => {
                // Rate limited
                rate_limit_response(retry_after)
            }
        }
    }
}

/// Result of rate limit check
pub enum RateLimitResult {
    Allowed,
    Limited(Response),
}

/// Helper to check rate limit via Durable Object
pub async fn check_r2_rate_limit(env: &Env, key: &str) -> Result<RateLimitResult> {
    // Get the rate limiter Durable Object namespace
    let namespace = match env.durable_object("R2_RATE_LIMITER") {
        Ok(ns) => ns,
        Err(_) => {
            // If rate limiter is not configured, allow the request
            console_log!(
                "R2_RATE_LIMITER Durable Object not configured, skipping rate limit check"
            );
            return Ok(RateLimitResult::Allowed);
        }
    };

    // Use a singleton Durable Object for rate limiting
    // In production, you might want to shard this based on key prefix
    let id = namespace.id_from_name("global-rate-limiter")?;
    let stub = id.get_stub()?;

    // Create request to check rate limit
    let check_url = format!("https://fake-host/check/{}", key);
    let request = Request::new(&check_url, Method::Get)?;

    let response = stub.fetch_with_request(request).await?;

    // Check if the request is allowed
    if response.status_code() == 200 {
        Ok(RateLimitResult::Allowed)
    } else {
        // Return the rate limit response from the Durable Object
        Ok(RateLimitResult::Limited(response))
    }
}
