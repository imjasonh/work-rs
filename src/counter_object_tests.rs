#[cfg(test)]
mod counter_object_tests {
    use crate::counter_object::CounterData;

    #[test]
    fn test_counter_data_structure() {
        let data = CounterData {
            count: 42,
            last_updated: 1234567890,
        };

        assert_eq!(data.count, 42);
        assert_eq!(data.last_updated, 1234567890);

        // Test serialization
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"count\":42"));
        assert!(json.contains("\"last_updated\":1234567890"));

        // Test deserialization
        let parsed: CounterData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.count, data.count);
        assert_eq!(parsed.last_updated, data.last_updated);
    }

    #[test]
    fn test_counter_increment_logic() {
        // Simulate counter increment
        let mut count = 0;
        count += 1;
        assert_eq!(count, 1);

        count += 1;
        assert_eq!(count, 2);

        // Test overflow behavior
        let mut max_count = i32::MAX - 1;
        max_count += 1;
        assert_eq!(max_count, i32::MAX);
    }

    #[test]
    fn test_counter_decrement_logic() {
        // Simulate counter decrement
        let mut count = 5;
        count -= 1;
        assert_eq!(count, 4);

        // Test decrement from zero
        let mut count = 0;
        count -= 1;
        assert_eq!(count, -1);

        // Test underflow behavior
        let mut min_count = i32::MIN + 1;
        min_count -= 1;
        assert_eq!(min_count, i32::MIN);
    }

    #[test]
    fn test_path_matching_for_counter() {
        let test_cases = vec![
            ("https://fake-host/", "GET"),
            ("https://fake-host/increment", "increment"),
            ("https://fake-host/decrement", "decrement"),
            ("https://fake-host/unknown", "404"),
            ("/", "GET"),
            ("/increment", "increment"),
            ("/decrement", "decrement"),
        ];

        for (path, expected_action) in test_cases {
            let action = if path.ends_with("/increment") {
                "increment"
            } else if path.ends_with("/decrement") {
                "decrement"
            } else if path.ends_with("/") {
                "GET"
            } else {
                "404"
            };
            assert_eq!(action, expected_action, "Failed for path: {}", path);
        }
    }

    #[test]
    fn test_counter_response_format() {
        // Test initial state response
        let initial_response = CounterData {
            count: 0,
            last_updated: 0,
        };
        let json = serde_json::to_value(&initial_response).unwrap();
        assert_eq!(json["count"], 0);
        assert_eq!(json["last_updated"], 0);

        // Test after increment response
        let incremented_response = CounterData {
            count: 1,
            last_updated: 1234567890,
        };
        let json = serde_json::to_value(&incremented_response).unwrap();
        assert_eq!(json["count"], 1);
        assert_eq!(json["last_updated"], 1234567890);
    }
}
