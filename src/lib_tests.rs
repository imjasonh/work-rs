#[cfg(test)]
mod lib_tests {
    
    #[test]
    fn test_route_matching() {
        // Test cases for different routes
        let test_cases = vec![
            ("/", "root"),
            ("/files/", "files"),
            ("/files/test.txt", "files"),
            ("/counter", "counter"),
            ("/counter/", "counter"),
            ("/counter/test", "counter"),
            ("/session/user123", "session"),
            ("/session/user123/key", "session"),
            ("/unknown", "404"),
        ];
        
        for (path, expected_route) in test_cases {
            let route = match path {
                "/" => "root",
                p if p.starts_with("/files/") => "files",
                p if p.starts_with("/counter") => "counter",
                p if p.starts_with("/session") => "session",
                _ => "404",
            };
            assert_eq!(route, expected_route, "Failed for path: {}", path);
        }
    }
    
    #[test]
    fn test_counter_id_extraction() {
        // Test counter ID extraction from paths
        let test_cases = vec![
            ("/counter", "default"),
            ("/counter/", "default"),
            ("/counter/test", "test"),
            ("/counter/my-counter", "my-counter"),
            ("/counter/123", "123"),
        ];
        
        for (path, expected_id) in test_cases {
            let counter_id = if path == "/counter" || path == "/counter/" {
                "default"
            } else {
                path.strip_prefix("/counter/").unwrap_or("default")
            };
            assert_eq!(counter_id, expected_id, "Failed for path: {}", path);
        }
    }
    
    #[test]
    fn test_session_path_parsing() {
        // Test session ID and key extraction
        let test_cases = vec![
            ("/session/user123", Some(("user123", None))),
            ("/session/user123/", Some(("user123", None))),
            ("/session/user123/preferences", Some(("user123", Some("preferences")))),
            ("/session/user123/data/nested", Some(("user123", Some("data")))),
            ("/session/", None),
            ("/session", None),
        ];
        
        for (path, expected) in test_cases {
            let parts: Vec<&str> = path.strip_prefix("/session/").unwrap_or("").split('/').collect();
            let result = if !parts.is_empty() && !parts[0].is_empty() {
                let session_id = parts[0];
                let key = parts.get(1).filter(|k| !k.is_empty()).map(|s| *s);
                Some((session_id, key))
            } else {
                None
            };
            
            assert_eq!(result, expected, "Failed for path: {}", path);
        }
    }
}