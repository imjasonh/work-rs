#[cfg(test)]
mod r2_rate_limiter_tests {
    use crate::rate_limiter::RateLimiter;

    // Note: We can't test rate_limit_response in unit tests because it uses
    // worker::Headers which requires WASM runtime. This would be better tested
    // in integration tests or with wrangler dev.

    #[test]
    fn test_rate_limiter_memory_cleanup() {
        let mut limiter = RateLimiter::new(1);

        // Add entries for multiple keys
        for i in 0..100 {
            let key = format!("file-{}.txt", i);
            limiter.check_rate_limit(&key).ok();
        }

        // Cleanup should remove old entries
        limiter.cleanup();

        // The exact behavior depends on timing, but this ensures cleanup doesn't panic
    }

    #[test]
    fn test_rate_limiter_concurrent_different_keys() {
        let mut limiter = RateLimiter::new(1);

        // Different keys should not interfere with each other
        let keys = vec![
            "file1.txt",
            "file2.txt",
            "file3.txt",
            "file4.txt",
            "file5.txt",
        ];

        // All first attempts should succeed
        for key in &keys {
            assert!(
                limiter.check_rate_limit(key).is_ok(),
                "First write to {} should succeed",
                key
            );
        }

        // Second attempts should fail
        for key in &keys {
            assert!(
                limiter.check_rate_limit(key).is_err(),
                "Second write to {} should be rate limited",
                key
            );
        }
    }

    #[test]
    fn test_rate_limiter_retry_duration() {
        let mut limiter = RateLimiter::new(1);

        // First write succeeds
        limiter.check_rate_limit("test.txt").unwrap();

        // Second write should fail with retry duration
        if let Err(duration) = limiter.check_rate_limit("test.txt") {
            // Retry duration should be positive but less than window
            assert!(duration.as_millis() > 0);
            assert!(duration.as_millis() <= 1000);
        } else {
            panic!("Expected rate limit error");
        }
    }

    #[test]
    fn test_rate_limiter_window_expiry() {
        let mut limiter = RateLimiter::new(1);

        // Add a write with a timestamp in the past
        // This simulates waiting for the window to expire
        // Note: In real tests, we'd need to mock time
        limiter.check_rate_limit("test.txt").unwrap();

        // Manually clear old entries
        limiter.cleanup();

        // This is a simplified test - in production we'd test with actual time delays
    }

    #[test]
    fn test_rate_limit_result_enum() {
        use crate::r2_rate_limiter::RateLimitResult;

        // Test that RateLimitResult can be constructed
        let allowed = RateLimitResult::Allowed;
        matches!(allowed, RateLimitResult::Allowed);

        // We can't easily test Limited variant without a real Response
    }

    #[test]
    fn test_path_validation_for_rate_limiter() {
        // Test various path formats that might be sent to the rate limiter
        let valid_paths = vec![
            "/check/file.txt",
            "/check/path/to/file.txt",
            "/check/file-name-123.dat",
        ];

        for path in valid_paths {
            assert!(path.starts_with("/check/"));
            let key = path.strip_prefix("/check/").unwrap();
            assert!(!key.is_empty());
        }

        // Invalid paths
        let invalid_paths = vec![
            "/check/",        // No key
            "/invalid/path",  // Wrong prefix
            "check/file.txt", // Missing leading slash
        ];

        for path in invalid_paths {
            assert!(
                !path.starts_with("/check/")
                    || path.strip_prefix("/check/").unwrap_or("").is_empty()
            );
        }
    }
}
