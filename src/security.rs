//! Security utilities for input validation and sanitization

/// Sanitize a file path to prevent directory traversal attacks
///
/// This function:
/// - Removes any `..` components that could escape the intended directory
/// - Removes leading slashes to ensure relative paths
/// - Rejects paths with null bytes
/// - Normalizes multiple slashes
pub fn sanitize_path(path: &str) -> Result<String, &'static str> {
    // Reject null bytes
    if path.contains('\0') {
        return Err("Invalid path: contains null byte");
    }

    // Split path into components and filter out dangerous ones
    let components: Vec<&str> = path
        .split('/')
        .filter(|component| !component.is_empty() && *component != "." && *component != "..")
        .collect();

    // Reject empty paths after sanitization
    if components.is_empty() {
        return Err("Invalid path: empty after sanitization");
    }

    // Reconstruct safe path
    Ok(components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path_normal() {
        assert_eq!(sanitize_path("file.txt").unwrap(), "file.txt");
        assert_eq!(sanitize_path("folder/file.txt").unwrap(), "folder/file.txt");
        assert_eq!(sanitize_path("a/b/c/file.txt").unwrap(), "a/b/c/file.txt");
    }

    #[test]
    fn test_sanitize_path_traversal() {
        assert_eq!(sanitize_path("../file.txt").unwrap(), "file.txt");
        assert_eq!(sanitize_path("../../etc/passwd").unwrap(), "etc/passwd");
        assert_eq!(
            sanitize_path("folder/../file.txt").unwrap(),
            "folder/file.txt"
        );
        assert_eq!(
            sanitize_path("./folder/./file.txt").unwrap(),
            "folder/file.txt"
        );
    }

    #[test]
    fn test_sanitize_path_slashes() {
        assert_eq!(sanitize_path("/file.txt").unwrap(), "file.txt");
        assert_eq!(sanitize_path("//file.txt").unwrap(), "file.txt");
        assert_eq!(
            sanitize_path("folder//file.txt").unwrap(),
            "folder/file.txt"
        );
        assert_eq!(
            sanitize_path("folder/file.txt/").unwrap(),
            "folder/file.txt"
        );
    }

    #[test]
    fn test_sanitize_path_invalid() {
        assert!(sanitize_path("").is_err());
        assert!(sanitize_path("/").is_err());
        assert!(sanitize_path("..").is_err());
        assert!(sanitize_path("../..").is_err());
        assert!(sanitize_path("file\0.txt").is_err());
    }

    #[test]
    fn test_sanitize_path_edge_cases() {
        assert_eq!(sanitize_path("...txt").unwrap(), "...txt"); // Three dots is valid
        assert_eq!(sanitize_path("..file").unwrap(), "..file"); // Two dots at start is valid
        assert_eq!(sanitize_path("file..").unwrap(), "file.."); // Two dots at end is valid
    }
}
