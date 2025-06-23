use colored::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Instant;

#[derive(Debug)]
struct TestCase {
    name: String,
    method: reqwest::Method,
    path: String,
    body: Option<String>,
    expected_status: u16,
    expected_content: Option<String>,
}

#[derive(Debug)]
struct TestResult {
    name: String,
    passed: bool,
    error: Option<String>,
    duration_ms: u128,
}

#[derive(Debug, Serialize, Deserialize)]
struct CounterResponse {
    count: i64,
    last_updated: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionData {
    user_id: String,
    data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileUploadResponse {
    key: String,
    size: u64,
    content_type: Option<String>,
    uploaded_at: i64,
    sha256: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = env::args()
        .nth(1)
        .unwrap_or_else(|| "http://localhost:8787".to_string());

    println!("🧪 Running E2E tests against: {}", base_url.cyan());
    println!();

    let timestamp = chrono::Utc::now().timestamp();
    let client = reqwest::Client::new();

    let test_cases = vec![
        // Basic connectivity
        TestCase {
            name: "GET / - Basic connectivity".to_string(),
            method: reqwest::Method::GET,
            path: "/".to_string(),
            body: None,
            expected_status: 200,
            expected_content: Some("Hello from Rust Workers".to_string()),
        },
        // Counter tests
        TestCase {
            name: "GET /counter - Initial state".to_string(),
            method: reqwest::Method::GET,
            path: "/counter".to_string(),
            body: None,
            expected_status: 200,
            expected_content: Some("count".to_string()),
        },
        TestCase {
            name: "POST /counter/increment".to_string(),
            method: reqwest::Method::POST,
            path: "/counter/increment".to_string(),
            body: None,
            expected_status: 200,
            expected_content: Some("count".to_string()),
        },
        TestCase {
            name: "DELETE /counter - Reset".to_string(),
            method: reqwest::Method::DELETE,
            path: "/counter".to_string(),
            body: None,
            expected_status: 200,
            expected_content: Some("Counter reset".to_string()),
        },
        // Session tests
        TestCase {
            name: format!("PUT /session/test-{} - Create session", timestamp),
            method: reqwest::Method::PUT,
            path: format!("/session/test-{}", timestamp),
            body: Some(r#"{"user_id":"test-user","data":{"theme":"dark"}}"#.to_string()),
            expected_status: 200,
            expected_content: Some("status".to_string()), // Response contains status field
        },
        TestCase {
            name: format!("GET /session/test-{} - Read session", timestamp),
            method: reqwest::Method::GET,
            path: format!("/session/test-{}", timestamp),
            body: None,
            expected_status: 200,
            expected_content: Some("test-user".to_string()),
        },
        TestCase {
            name: format!("DELETE /session/test-{} - Delete session", timestamp),
            method: reqwest::Method::DELETE,
            path: format!("/session/test-{}", timestamp),
            body: None,
            expected_status: 200,
            expected_content: Some("Session cleared".to_string()),
        },
        // R2 storage tests
        TestCase {
            name: format!("PUT /files/test-{}.txt - Upload file", timestamp),
            method: reqwest::Method::PUT,
            path: format!("/files/test-{}.txt", timestamp),
            body: Some(format!("Hello from E2E test at {}", timestamp)),
            expected_status: 200,
            expected_content: Some("sha256".to_string()),
        },
        TestCase {
            name: format!("GET /files/test-{}.txt - Download file", timestamp),
            method: reqwest::Method::GET,
            path: format!("/files/test-{}.txt", timestamp),
            body: None,
            expected_status: 200,
            expected_content: Some(format!("Hello from E2E test at {}", timestamp)),
        },
        TestCase {
            name: "GET /files/ - List files (empty path)".to_string(),
            method: reqwest::Method::GET,
            path: "/files/".to_string(),
            body: None,
            expected_status: 400, // Empty path after sanitization
            expected_content: None,
        },
        TestCase {
            name: format!("DELETE /files/test-{}.txt - Delete file", timestamp),
            method: reqwest::Method::DELETE,
            path: format!("/files/test-{}.txt", timestamp),
            body: None,
            expected_status: 200,
            expected_content: Some("File deleted".to_string()),
        },
        // Security tests - path traversal
        // Note: These return 404 because the router doesn't match the path pattern
        TestCase {
            name: "Security: GET /files/../etc/passwd".to_string(),
            method: reqwest::Method::GET,
            path: "/files/../etc/passwd".to_string(),
            body: None,
            expected_status: 404, // Router rejects before reaching file handler
            expected_content: None,
        },
        TestCase {
            name: "Security: PUT /files/../../etc/passwd".to_string(),
            method: reqwest::Method::PUT,
            path: "/files/../../etc/passwd".to_string(),
            body: Some("malicious content".to_string()),
            expected_status: 404, // Router rejects before reaching file handler
            expected_content: None,
        },
        TestCase {
            name: "Security: GET /session/../../../etc/passwd".to_string(),
            method: reqwest::Method::GET,
            path: "/session/../../../etc/passwd".to_string(),
            body: None,
            expected_status: 404, // Router rejects before reaching session handler
            expected_content: None,
        },
    ];

    let mut results = Vec::new();

    println!("{}", "=== Running Tests ===".bold());
    for test_case in test_cases {
        let result = run_test(&client, &base_url, test_case).await?;

        let status_icon = if result.passed {
            "✓".green()
        } else {
            "✗".red()
        };
        let status_text = if result.passed {
            "PASSED".green()
        } else {
            "FAILED".red()
        };

        println!(
            "{} {} - {} ({}ms)",
            status_icon, result.name, status_text, result.duration_ms
        );

        if let Some(error) = &result.error {
            println!("  {}: {}", "Error".red(), error);
        }

        results.push(result);
    }

    println!();
    println!("{}", "=== Test Summary ===".bold());

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();

    println!("Tests passed: {}", passed.to_string().green());
    println!("Tests failed: {}", failed.to_string().red());

    if failed > 0 {
        println!();
        println!("{}", "Failed tests:".red().bold());
        for result in results.iter().filter(|r| !r.passed) {
            println!("  - {}", result.name);
            if let Some(error) = &result.error {
                println!("    {}", error);
            }
        }
        std::process::exit(1);
    } else {
        println!();
        println!("{}", "All tests passed! 🎉".green().bold());
    }

    Ok(())
}

async fn run_test(
    client: &reqwest::Client,
    base_url: &str,
    test_case: TestCase,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    let url = format!("{}{}", base_url, test_case.path);
    let start = Instant::now();

    let mut request = client.request(test_case.method, &url);

    if let Some(body) = &test_case.body {
        request = request
            .header("Content-Type", "application/json")
            .body(body.clone());
    }

    let response = request.send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;

    let duration_ms = start.elapsed().as_millis();

    let status_matches = status == test_case.expected_status;
    let content_matches = test_case
        .expected_content
        .as_ref()
        .map_or(true, |expected| body.contains(expected));

    let passed = status_matches && content_matches;

    let error = if !passed {
        Some(format!(
            "Status: expected {}, got {}. Content: {}",
            test_case.expected_status,
            status,
            if !content_matches {
                format!(
                    "expected '{}' in response",
                    test_case.expected_content.unwrap_or_default()
                )
            } else {
                "ok".to_string()
            }
        ))
    } else {
        None
    };

    Ok(TestResult {
        name: test_case.name,
        passed,
        error,
        duration_ms,
    })
}
